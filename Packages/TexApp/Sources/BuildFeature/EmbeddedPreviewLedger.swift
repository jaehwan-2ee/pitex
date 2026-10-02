/// `EmbeddedPreviewLedger` — pure bookkeeping for the embedded editing
/// preview session (`PreviewEngine/PROTOCOL.md`). No I/O: the workspace
/// streams updates and helper events through it and performs the releases
/// it returns. Same semantics as the Linux client
/// (`pitex-shell/src/embedded_preview.rs`).
///
/// Two counters:
/// - `generation` numbers transport updates. Saves, closes and rescans send
///   updates too; it stays monotonic across helper restarts.
/// - `editRevision` counts real source edits, bumped when the edit happens,
///   so an edit still waiting in the coalescing window already counts.
///
/// The manual-final floor is an edit revision: an update sent after a manual
/// build started — a queued pre-build edit flushed late, the save-only
/// update its pre-save causes — never puts a preview over the final PDF;
/// only an edit made after the build started (also while it runs) does.
public struct EmbeddedPreviewLedger: Equatable, Sendable {
    /// Sent updates remembered for floors and SyncTeX validation.
    public static let historyLimit = 32

    /// Source identity: absolute path → content hash.
    public typealias Sources = [String: UInt64]

    public struct SentUpdate: Equatable, Sendable {
        public let generation: UInt64
        /// Newest edit revision whose text this update carries.
        public let revision: UInt64
        /// Every open TeX source as compiled for this generation — dirty
        /// buffers with their unsaved text, clean ones with their (disk)
        /// text. A save keeps the identity; any divergence breaks it.
        public let sources: Sources
    }

    public struct Displayed: Equatable, Sendable {
        public let seq: UInt64
        /// Newest generation this publication is current for (`idle`
        /// advances it without new artifacts).
        public fileprivate(set) var generation: UInt64
        /// Edit revision of the generation it was compiled for; 0 when that
        /// generation had already left the history.
        public let revision: UInt64
        public let complete: Bool
    }

    public enum Acceptance: Equatable, Sendable {
        /// Show it; release the publication it replaced, if any, once the
        /// display and SyncTeX binding moved.
        case display(releasing: UInt64?)
        /// Not newer than what is on screen, or never sent — release it.
        case stale
        /// Carries no edit newer than the last final build — release it;
        /// the final PDF stays.
        case belowFinalFloor
    }

    public enum FinalOutcome: Equatable, Sendable {
        /// Show the final PDF; release the displayed preview, if any.
        case showFinal(releasing: UInt64?)
        /// The displayed preview already carries an edit made after the
        /// build started — drafting continues, the final PDF is not shown.
        case keepPreview
    }

    public private(set) var generation: UInt64 = 0
    public private(set) var editRevision: UInt64 = 0
    public private(set) var displayed: Displayed?
    public private(set) var finalFloor: UInt64?
    private var pendingFloor: UInt64?
    private var history: [SentUpdate] = []

    public init() {}

    /// A real source edit (a content change of an open TeX buffer).
    public mutating func noteEdit() {
        editRevision += 1
    }

    /// Records the update about to be queued and returns its generation.
    /// `revision` is the edit revision the collected texts correspond to.
    /// (Which override texts go over the pipe is `EmbeddedPreviewOutbox`'s
    /// business: a queued update may be replaced before it is written.)
    public mutating func recordUpdate(sources: Sources, revision: UInt64) -> UInt64 {
        generation += 1
        history.append(SentUpdate(generation: generation, revision: min(revision, editRevision), sources: sources))
        if history.count > Self.historyLimit { history.removeFirst(history.count - Self.historyLimit) }
        return generation
    }

    public func update(_ generation: UInt64) -> SentUpdate? {
        history.last { $0.generation == generation }
    }

    /// Display rule: newer seq, not an older generation than the display,
    /// and — once a final build set a floor — an edit newer than it. It
    /// may lag the newest sent generation.
    public mutating func accept(seq: UInt64, generation: UInt64, complete: Bool) -> Acceptance {
        guard generation <= self.generation else { return .stale }
        if let current = displayed, seq <= current.seq || generation < current.generation {
            return .stale
        }
        let revision = update(generation)?.revision
        if let floor = finalFloor, (revision ?? 0) <= floor { return .belowFinalFloor }
        let previous = displayed?.seq
        displayed = Displayed(seq: seq, generation: generation, revision: revision ?? 0, complete: complete)
        return .display(releasing: previous)
    }

    /// `idle`: publication `seq` is current for `generation` too. True when
    /// that moved the displayed publication's source identity forward.
    public mutating func noteIdle(generation: UInt64, seq: UInt64) -> Bool {
        guard var current = displayed, current.seq == seq,
              generation > current.generation, generation <= self.generation else { return false }
        current.generation = generation
        displayed = current
        return true
    }

    /// A manual build is about to snapshot its sources: every edit noted so
    /// far — including ones still waiting to be sent — is part of it.
    public mutating func noteFinalBuildStarted() {
        pendingFloor = editRevision
    }

    /// The manual build succeeded.
    public mutating func noteFinalBuildPublished() -> FinalOutcome {
        let floor = pendingFloor ?? editRevision
        pendingFloor = nil
        finalFloor = max(finalFloor ?? floor, floor)
        guard let current = displayed else { return .showFinal(releasing: nil) }
        if current.revision > floor { return .keepPreview }
        displayed = nil
        return .showFinal(releasing: current.seq)
    }

    /// The helper stopped or restarted: its publications and history are
    /// gone. Counters and floors survive (the final PDF on screen still
    /// wins over previews without newer edits).
    public mutating func resetSession() {
        history = []
        displayed = nil
    }

    /// Workspace close or build-target change: the final PDF the floor
    /// protected is no longer this context's output.
    public mutating func resetContext() {
        resetSession()
        finalFloor = nil
        pendingFloor = nil
    }
}
