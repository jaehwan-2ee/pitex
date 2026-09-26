/// `LiveCompileScheduler` — the pure, deterministic state machine behind
/// live auto compile. No threads, timers or sleeps: the caller owns the
/// clock and feeds millisecond timestamps in. Every event returns the
/// ``LiveRequest``s the caller must perform (start/cancel); nothing else
/// happens inside.
///
/// Integration contract:
///
/// - `noteEdit(nowMs:)` on every real source mutation (also while IME
///   composition is in progress — the deadline just keeps moving).
/// - `poll(nowMs:composing:)` when the caller's debounce timer fires (arm it
///   for `pendingDeadline`) and whenever work may have been freed (it is
///   idempotent).
/// - `requestManual()` when the user starts a manual build.
/// - `completed(token:nowMs:composing:)` exactly once for every started
///   run — including runs that never launched a process — so the slot is
///   released and queued work can dispatch.
/// - `invalidate()` on workspace switch/close or build-target/command
///   changes; `setEnabled(false)` on the toggle. Both retire only live
///   work — an active manual run keeps ownership.
///
/// Stale-result suppression: a live completion is reported
/// ``LiveCompletion/superseded(_:)`` when newer edits were recorded, an
/// invalidation bumped the generation, or the run was asked to cancel —
/// the UI clears its busy state but must not publish the artifact, log,
/// issues or SyncTeX binding. A token that is not the active run is
/// ``LiveCompletion/stale`` and changes nothing.
public struct LiveCompileScheduler: Equatable, Sendable {
    /// Default idle debounce (matches `liveCompileDelayMilliseconds`).
    public static let defaultDelayMs: UInt64 = 700
    /// Settings clamp bounds — callers clamp before `setDelay`.
    public static let minDelayMs: UInt64 = 200
    public static let maxDelayMs: UInt64 = 10_000
    /// Remote workspaces never debounce shorter than this — the upload and
    /// device round-trip dominate anything smaller.
    public static let remoteMinDelayMs: UInt64 = 1_500

    public private(set) var enabled: Bool
    public private(set) var delayMs: UInt64
    /// Bumped by `invalidate()`/toggle-off; older-generation work is dead.
    private var generation: UInt64
    private var nextId: UInt64
    private var pending: PendingEdit?
    private var activeRun: ActiveRun?
    /// A manual build asked for while a live run still holds the slot.
    private var queuedManual: Bool

    private struct PendingEdit: Equatable, Sendable {
        /// Millisecond timestamp of the newest edit (deadline base, so a
        /// later `setDelay` re-derives from the edit, not from "now").
        var editedAtMs: UInt64
        var deadlineMs: UInt64
        var generation: UInt64
    }

    private struct ActiveRun: Equatable, Sendable {
        var token: LiveRunToken
        /// A cancel request was already emitted — never emit another.
        var cancelRequested: Bool
    }

    public init() {
        enabled = false
        delayMs = Self.defaultDelayMs
        generation = 0
        nextId = 0
        pending = nil
        activeRun = nil
        queuedManual = false
    }

    /// Deadline the caller's timer should fire at, if an edit is pending.
    public var pendingDeadline: UInt64? {
        pending?.deadlineMs
    }

    /// The run currently holding the slot.
    public var active: LiveRunToken? {
        activeRun?.token
    }

    /// True while a live build owns the slot (also when cancelling).
    public var liveActive: Bool {
        activeRun?.token.kind == .live
    }

    /// True while a manual build owns the slot.
    public var manualActive: Bool {
        activeRun?.token.kind == .manual
    }

    /// Edits or a queued manual wait behind the active run.
    public var hasPendingWork: Bool {
        pending != nil || queuedManual
    }

    /// May events/results stamped with `token` be applied to the UI now?
    /// For a live run: only while it is still current — once an edit,
    /// manual request or invalidation asked it to cancel, its streaming
    /// log/issue/binding events are stale and must be dropped (busy state
    /// still clears at `completed`). A manual run's results are accepted
    /// while it is active even across `invalidate()`/`setEnabled(false)`
    /// generation bumps — manual work keeps ownership of its results.
    /// The caller separately guards workspace identity for asynchronous
    /// SyncTeX results landing after completion.
    public func acceptsActiveResult(_ token: LiveRunToken) -> Bool {
        guard let active = activeRun, active.token == token else { return false }
        switch token.kind {
        case .manual:
            return true
        case .live:
            return !active.cancelRequested && token.generation == generation
        }
    }

    /// Toggle. Off retires live work: pending edits drop, the generation
    /// moves and only a live run is asked to cancel — a running manual
    /// build keeps ownership and an explicitly queued manual survives
    /// (only `invalidate()` drops it). On does not schedule anything; the
    /// next edit does.
    @discardableResult
    public mutating func setEnabled(_ enabled: Bool) -> [LiveRequest] {
        guard self.enabled != enabled else { return [] }
        self.enabled = enabled
        return enabled ? [] : retireLiveWork()
    }

    /// New debounce delay (already clamped by the caller). A pending edit
    /// re-derives its deadline from its edit timestamp, so shortening the
    /// delay fires earlier rather than sliding a new full delay.
    public mutating func setDelay(_ delayMs: UInt64) {
        self.delayMs = delayMs
        if let editedAtMs = pending?.editedAtMs {
            pending?.deadlineMs = editedAtMs.saturatingAdd(delayMs)
        }
    }

    /// Workspace switch/close or target/command change: retire all live
    /// work. Pending edits and a queued manual are dropped (they belonged
    /// to the old context); an active manual run keeps ownership.
    @discardableResult
    public mutating func invalidate() -> [LiveRequest] {
        let requests = retireLiveWork()
        queuedManual = false
        return requests
    }

    /// A real source edit: record the newest deadline and cancel a
    /// superseded live run immediately. During a manual run the edit just
    /// stays pending — manual builds keep priority.
    @discardableResult
    public mutating func noteEdit(nowMs: UInt64) -> [LiveRequest] {
        guard enabled else { return [] }
        pending = PendingEdit(
            editedAtMs: nowMs,
            deadlineMs: nowMs.saturatingAdd(delayMs),
            generation: generation
        )
        var requests: [LiveRequest] = []
        cancelActiveLive(&requests)
        return requests
    }

    /// The user asked for a manual build. Manual work covers the current
    /// source, so any pending live edit is consumed; a running live build
    /// is cancelled and the manual run queues behind it — never overlaps.
    @discardableResult
    public mutating func requestManual() -> [LiveRequest] {
        pending = nil
        queuedManual = true
        var requests: [LiveRequest] = []
        cancelActiveLive(&requests)
        dispatchManual(&requests)
        return requests
    }

    /// Timer poll: emit the pending live start once its deadline passed,
    /// the slot is free and no IME composition is in progress. Emits at
    /// most one start per poll.
    @discardableResult
    public mutating func poll(nowMs: UInt64, composing: Bool) -> [LiveRequest] {
        var requests: [LiveRequest] = []
        dispatch(nowMs: nowMs, composing: composing, into: &requests)
        return requests
    }

    /// A started run finished (any lifecycle, including never-launched).
    /// Frees the slot and dispatches whatever queued behind it — a queued
    /// manual first, then a live edit whose deadline already elapsed (no
    /// second debounce wait after a cancelled run).
    @discardableResult
    public mutating func completed(
        token: LiveRunToken,
        nowMs: UInt64,
        composing: Bool
    ) -> (status: LiveCompletion, requests: [LiveRequest]) {
        let status: LiveCompletion
        if let active = activeRun, active.token == token {
            let superseded: Bool
            switch token.kind {
            case .live:
                superseded = active.cancelRequested
                    || pending != nil
                    || token.generation != generation
            case .manual:
                // A manual result may display even with newer input
                // pending — the pending live run then builds latest.
                superseded = false
            }
            activeRun = nil
            status = superseded ? .superseded(token) : .current(token)
        } else {
            // An obsolete completion changes nothing — in particular it
            // must not dispatch a due pending edit (the caller's timer
            // owns that).
            return (.stale, [])
        }
        var requests: [LiveRequest] = []
        dispatch(nowMs: nowMs, composing: composing, into: &requests)
        return (status, requests)
    }

    /// Shared tail of `setEnabled(false)`/`invalidate()`: bump the
    /// generation, drop pending edits and ask an active live run to
    /// cancel. Keeps `queuedManual` — `invalidate()` drops it afterwards
    /// because a queued manual belonged to the old context, while
    /// toggle-off must not eat the user's explicit build request.
    private mutating func retireLiveWork() -> [LiveRequest] {
        generation += 1
        pending = nil
        var requests: [LiveRequest] = []
        cancelActiveLive(&requests)
        return requests
    }

    private mutating func mint(kind: LiveRunKind) -> LiveRunToken {
        nextId += 1
        return LiveRunToken(id: nextId, kind: kind, generation: generation)
    }

    private mutating func cancelActiveLive(_ requests: inout [LiveRequest]) {
        guard let active = activeRun,
              active.token.kind == .live,
              !active.cancelRequested
        else { return }
        activeRun?.cancelRequested = true
        requests.append(.cancel(token: active.token))
    }

    private mutating func dispatchManual(_ requests: inout [LiveRequest]) {
        guard activeRun == nil, queuedManual else { return }
        queuedManual = false
        let token = mint(kind: .manual)
        activeRun = ActiveRun(token: token, cancelRequested: false)
        requests.append(.startManual(token: token))
    }

    private mutating func dispatch(
        nowMs: UInt64,
        composing: Bool,
        into requests: inout [LiveRequest]
    ) {
        guard activeRun == nil else { return }
        if queuedManual {
            dispatchManual(&requests)
            return
        }
        guard let pendingEdit = pending,
              enabled,
              !composing,
              nowMs >= pendingEdit.deadlineMs,
              pendingEdit.generation == generation
        else { return }
        pending = nil
        let token = mint(kind: .live)
        activeRun = ActiveRun(token: token, cancelRequested: false)
        requests.append(.startLive(token: token, deadlineMs: pendingEdit.deadlineMs))
    }
}

/// The two owners of the single build slot.
public enum LiveRunKind: Hashable, Sendable {
    case live
    case manual
}

/// Identity of one started run; completions echo it back verbatim.
public struct LiveRunToken: Hashable, Sendable, CustomStringConvertible {
    /// Unique per scheduler — ordering is never inferred from it.
    public let id: UInt64
    public let kind: LiveRunKind
    /// `invalidate()` generation at start; a lower one is obsolete.
    public let generation: UInt64

    public init(id: UInt64, kind: LiveRunKind, generation: UInt64) {
        self.id = id
        self.kind = kind
        self.generation = generation
    }

    public var description: String {
        let name = kind == .live ? "live" : "manual"
        return "\(name)-\(id)-g\(generation)"
    }
}

/// What the scheduler asks the caller to perform.
public enum LiveRequest: Equatable, Sendable {
    /// Start a live build covering the newest pending edit.
    /// `deadlineMs` echoes the debounce deadline that fired.
    case startLive(token: LiveRunToken, deadlineMs: UInt64)
    /// Start the requested manual build.
    case startManual(token: LiveRunToken)
    /// Cancel the in-flight run — always a live one; the scheduler never
    /// cancels manual work.
    case cancel(token: LiveRunToken)
}

/// How `completed` classified the finishing run.
public enum LiveCompletion: Equatable, Sendable {
    /// The finishing run is current — publish its result.
    case current(LiveRunToken)
    /// The run finished but was superseded (newer edits, invalidation or
    /// a manual request): clear busy state, suppress its artifacts.
    case superseded(LiveRunToken)
    /// Not the active run — an obsolete or unknown completion. Nothing
    /// changed; the newer active run keeps its slot.
    case stale
}

private extension UInt64 {
    func saturatingAdd(_ other: UInt64) -> UInt64 {
        let (sum, overflow) = addingReportingOverflow(other)
        return overflow ? .max : sum
    }
}
