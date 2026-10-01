import LanguageCore

/// `EquationPreviewEngine` — the pure, deterministic state machine behind
/// the Overleaf-style equation preview (mirrored by the Rust
/// `editor_feature::equation_preview`). No threads, timers or I/O: the host
/// feeds editor events with millisecond timestamps, arms one timer for
/// ``nextDeadline`` and calls ``poll(nowMs:text:)`` when it fires. Every
/// entry point returns the ``EquationPreviewCommand``s the host performs.
///
/// Integration contract:
///
/// - `textChanged`/`selectionChanged` carry the host's text revision; a
///   selection event at an unchanged revision is navigation, not typing.
/// - Offsets are the host's text units (UTF-16 here, `NSRange`); `poll`
///   receives the current text lazily so idle events never copy it.
/// - Exactly one fast render is in flight; the host answers every
///   `.render` with `fastRenderCompleted` and every `.renderExact` with
///   `exactRenderCompleted` (also after `.cancelExact`).
/// - A result is published only while its generation is the newest one
///   (a fire for the same cache key adopts the in-flight request).
public struct EquationPreviewEngine {
    public static let keepLastGoodMs: UInt64 = 300
    public static let hoverExitGraceMs: UInt64 = 250
    public static let exactSettleMs: UInt64 = 600
    public static let caretDelayCapMs: UInt64 = 30
    public static let maximumSourceBytes = 32 * 1024
    public static let cacheCapacity = 256
    public static let cacheByteBudget = 16 * 1024 * 1024
    public static let exactCacheCapacity = 16
    public static let failureCacheCapacity = 128

    public private(set) var settings: EquationPreviewSettings
    public private(set) var appearance: EquationPreviewAppearance
    /// Renderer identity (MathJax + font + page revision) once the page is
    /// ready; renders wait for it, and a change empties the cache.
    public private(set) var rendererVersion: String?
    public private(set) var rendererFailed = false
    /// Exact renderer identity (engine + shell-escape policy); nil when the
    /// project build command names no supported TeX engine.
    public private(set) var exactProfile: String?

    private var fileID: String?
    private var rootFileID: String?
    private var textRevision: UInt64 = 0
    private var snapshot: Snapshot?
    private var externalScans: [String: MathSourceScan] = [:]
    private var resolutions: [MathIncludeKey: String?] = [:]
    private var pendingResolutions = Set<MathIncludeKey>()
    private var contextVersion: UInt64 = 0
    private var projectContext: (version: UInt64, revision: UInt64, context: MathProjectContext)?

    private var focused = false
    private var composing = false
    private var selection = (location: 0, length: 0)
    private var selectionRevision: UInt64?
    private var suppressedByTyping = false
    private var hoverLocation: Int?
    private var pointerInPopover = false
    private var dismissedRegionStart: Int?
    private var explicitExactRegionStart: Int?

    private var generation: UInt64 = 0
    private var fireDeadline: UInt64?
    private var keepDeadline: UInt64?
    private var hoverGraceDeadline: UInt64?
    private var exactDeadline: UInt64?

    private var target: Target?
    private var visible: Visible?
    private var inFlight: (generation: UInt64, key: EquationPreviewKey)?
    private var queued: EquationRenderRequest?
    private var exactInFlight: (generation: UInt64, key: String)?
    private var cache = EquationPreviewLRU<EquationPreviewKey, String>(capacity: cacheCapacity, byteBudget: cacheByteBudget)
    private var failures = EquationPreviewLRU<EquationPreviewKey, EquationRenderFailure>(capacity: failureCacheCapacity, byteBudget: .max)
    private var exactCache = EquationPreviewLRU<String, [UInt8]>(capacity: exactCacheCapacity, byteBudget: 8 * 1024 * 1024)

    private struct Snapshot {
        let revision: UInt64
        let fileID: String
        /// Native UTF-8 (`makeContiguousUTF8`), so both views index in O(1)
        /// amortized via String's UTF-16 breadcrumbs. (C11 r2: the
        /// per-revision boundary table was reverted — its O(chars) build
        /// per edit plus linear lookups regressed vs breadcrumbs.)
        let text: String
        let scan: MathSourceScan

        /// Host UTF-16 offset -> UTF-8 byte offset (clamped).
        func utf8(forUTF16 offset: Int) -> Int {
            let utf16 = text.utf16
            var index = utf16.index(utf16.startIndex, offsetBy: max(0, min(offset, utf16.count)))
            // A trailing surrogate half rounds down to its scalar's start.
            if index.samePosition(in: text.unicodeScalars) == nil { index = utf16.index(before: index) }
            return text.utf8.distance(from: text.utf8.startIndex, to: index)
        }

        /// UTF-8 byte offset -> host UTF-16 offset.
        func utf16(forUTF8 offset: Int) -> Int {
            let utf8 = text.utf8
            let index = utf8.index(utf8.startIndex, offsetBy: max(0, min(offset, utf8.count)))
            return text.utf16.distance(from: text.utf16.startIndex, to: index)
        }
    }

    private struct Target {
        var generation: UInt64
        let region: MathRegion
        let anchor: Range<Int>
        let trigger: EquationPreviewTrigger
        let key: EquationPreviewKey?
        let exact: ExactPlan?
        var failure: EquationRenderFailure?
    }

    private struct ExactPlan {
        let key: String
        let document: String
    }

    private struct Visible: Equatable {
        let regionStart: Int
        let content: EquationPreviewPresentation.Content
    }

    public init(settings: EquationPreviewSettings = EquationPreviewSettings(), appearance: EquationPreviewAppearance = EquationPreviewAppearance()) {
        self.settings = settings
        self.appearance = appearance
    }

    /// Earliest instant `poll` must run.
    public var nextDeadline: UInt64? {
        [fireDeadline, keepDeadline, hoverGraceDeadline, exactDeadline].compactMap { $0 }.min()
    }

    /// Something is on screen (the host consumes Escape only then).
    public var isVisible: Bool { visible != nil }

    /// Files the context depends on besides the active one — the host
    /// re-checks them (off the main thread) after saves or app activation.
    public var externalFileIDs: [String] { externalScans.keys.sorted() }

    // MARK: Configuration

    public mutating func setSettings(_ settings: EquationPreviewSettings, nowMs: UInt64) -> [EquationPreviewCommand] {
        guard settings != self.settings else { return [] }
        let modeChanged = settings.renderer != self.settings.renderer
        self.settings = settings
        guard settings.enabled else { return retire() }
        if modeChanged { failures.removeAll() }
        // Only the transition TO .fast kills exact work (C8 r2):
        // fastWithTeXFallback -> fast leaves a running job whose result
        // could still present; fast -> fastWithTeXFallback keeps it useful.
        if modeChanged, settings.renderer == .fast {
            explicitExactRegionStart = nil
            exactDeadline = nil
            let commands = cancelExact()
            schedule(at: nowMs)
            return commands
        }
        schedule(at: nowMs)
        return []
    }

    public mutating func setAppearance(_ appearance: EquationPreviewAppearance, nowMs: UInt64) -> [EquationPreviewCommand] {
        guard appearance != self.appearance else { return [] }
        self.appearance = appearance
        schedule(at: nowMs)
        return []
    }

    public mutating func rendererReady(version: String, nowMs: UInt64) -> [EquationPreviewCommand] {
        if version != rendererVersion {
            cache.removeAll()
            failures.removeAll()
        }
        rendererVersion = version
        rendererFailed = false
        schedule(at: nowMs)
        return []
    }

    /// The renderer page could not load (or its process died); fast previews
    /// stop until the host reports a fresh `rendererReady`.
    public mutating func rendererUnavailable() -> [EquationPreviewCommand] {
        rendererVersion = nil
        rendererFailed = true
        inFlight = nil
        queued = nil
        return retire()
    }

    public mutating func setExactProfile(_ profile: String?, nowMs: UInt64) -> [EquationPreviewCommand] {
        guard profile != exactProfile else { return [] }
        exactProfile = profile
        exactCache.removeAll()
        failures.removeAll()
        let commands = cancelExact()
        schedule(at: nowMs)
        return commands
    }

    // MARK: Document

    /// Document switch (or first open). The outgoing file's last scan keeps
    /// serving as its context until the host reports a fresher one.
    public mutating func openDocument(fileID: String?, revision: UInt64, nowMs: UInt64) -> [EquationPreviewCommand] {
        if let snapshot, snapshot.fileID != fileID {
            externalScans[snapshot.fileID] = snapshot.scan
        }
        if let fileID { externalScans[fileID] = nil }
        contextVersion += 1
        self.fileID = fileID
        textRevision = revision
        snapshot = nil
        hoverLocation = nil
        selectionRevision = nil
        suppressedByTyping = false
        dismissedRegionStart = nil
        explicitExactRegionStart = nil
        let commands = retire()
        schedule(at: nowMs)
        return commands
    }

    /// The main document the context and exact preamble start from; nil
    /// uses the active file as its own root.
    public mutating func setRootFile(_ rootFileID: String?, scan: MathSourceScan?, nowMs: UInt64) -> [EquationPreviewCommand] {
        if let rootFileID, rootFileID != fileID { externalScans[rootFileID] = scan }
        guard rootFileID != self.rootFileID || scan != nil else { return [] }
        self.rootFileID = rootFileID
        contextVersion += 1
        // A changed root/preamble changes exact-render semantics without
        // changing any document text or cache key: stale PDFs must go (C5).
        exactCache.removeAll()
        schedule(at: nowMs)
        return []
    }
    public mutating func textChanged(revision: UInt64, nowMs: UInt64) -> [EquationPreviewCommand] {
        textRevision = revision
        dismissedRegionStart = nil
        explicitExactRegionStart = nil
        exactDeadline = nil
        // A stale hover anchor must not survive the edit: the character
        // index now points at different text, and a hover-target preview
        // would render a region the pointer no longer selects (B3).
        hoverLocation = nil
        hoverGraceDeadline = nil
        if target?.trigger == .hover { target = nil }
        var commands: [EquationPreviewCommand] = []
        if !settings.whileTyping {
            suppressedByTyping = true
            commands += retire()
        }
        schedule(at: nowMs + settings.delayMilliseconds)
        return commands
    }

    /// Hosts report the post-edit caret with the edit's new revision (before
    /// or after `textChanged`); only a selection change at an unchanged
    /// revision counts as navigation and ends typing suppression.
    public mutating func selectionChanged(location: Int, length: Int, revision: UInt64, nowMs: UInt64) -> [EquationPreviewCommand] {
        let navigation = selectionRevision == revision && revision == textRevision
        selectionRevision = revision
        let resumed = navigation && suppressedByTyping
        if navigation { suppressedByTyping = false }
        guard (location, length) != selection || resumed else { return [] }
        selection = (location, length)
        // Never shorten a pending typing debounce.
        schedule(at: max(fireDeadline ?? 0, nowMs + min(settings.delayMilliseconds, Self.caretDelayCapMs)))
        return []
    }

    // MARK: Pointer / focus

    /// The character under the pointer, or nil when the pointer is outside
    /// the text (or a button is held). Hover never moves the caret.
    public mutating func hover(location: Int?, nowMs: UInt64) -> [EquationPreviewCommand] {
        if let location {
            // Grace is NOT cleared here: it counts down from the actual
            // exit so motions over plain text cannot stretch it
            // indefinitely (C6). fire() clears it on a math region hit.
            guard location != hoverLocation else { return [] }
            hoverLocation = location
            schedule(at: nowMs + settings.delayMilliseconds)
        } else if target?.trigger == .hover {
            // Pointer left the text entirely: the position is gone, so the
            // stale index must not be consulted at re-fire — the grace
            // deadline alone protects the visible popover (C6 r2).
            hoverLocation = nil
            if hoverGraceDeadline == nil { hoverGraceDeadline = nowMs + Self.hoverExitGraceMs }
        } else if hoverLocation != nil {
            hoverLocation = nil
            schedule(at: nowMs)
        }
        return []
    }

    public mutating func pointerInPopover(_ inside: Bool, nowMs: UInt64) -> [EquationPreviewCommand] {
        pointerInPopover = inside
        if inside {
            hoverGraceDeadline = nil
        } else if target?.trigger == .hover {
            hoverGraceDeadline = nowMs + Self.hoverExitGraceMs
        } else {
            // Leaving the popover for a non-editor area must re-evaluate:
            // a caret-triggered target with no hoverLocation has no other
            // event that would ever hide it (C7).
            schedule(at: nowMs)
        }
        return []
    }

    public mutating func focusChanged(_ focused: Bool, nowMs: UInt64) -> [EquationPreviewCommand] {
        guard focused != self.focused else { return [] }
        self.focused = focused
        guard focused else {
            hoverLocation = nil
            pointerInPopover = false
            hoverGraceDeadline = nil
            return retire()
        }
        schedule(at: nowMs)
        return []
    }

    /// IME marked text: previews stay down until composition ends.
    public mutating func compositionChanged(_ composing: Bool, nowMs: UInt64) -> [EquationPreviewCommand] {
        guard composing != self.composing else { return [] }
        self.composing = composing
        if composing { return retire() }
        schedule(at: nowMs + settings.delayMilliseconds)
        return []
    }

    /// Escape: dismisses a visible preview, or a pending-but-eligible one
    /// (scheduled fire resolving to math, or a live render request).
    /// `consumed` tells the host whether to swallow the key; it is false
    /// for dormant/failed targets, pending fires on plain text, and any
    /// stale work — none of which may be poisoned by a dismissal (N5).
    public mutating func escape(nowMs: UInt64) -> (consumed: Bool, commands: [EquationPreviewCommand]) {
        // Pending fire is eligible only when it would resolve to a math
        // region. The answer needs the *current* scan; with a stale or
        // absent snapshot we decline rather than poison a dormant target.
        var pendingRegionStart: Int?
        if fireDeadline != nil, settings.enabled, !composing,
           !(rendererVersion == nil && rendererFailed),
           let snapshot, snapshot.revision == textRevision, snapshot.fileID == fileID {
            var region: MathRegion?
            if let hoverLocation {
                region = snapshot.scan.region(atCharacter: snapshot.utf8(forUTF16: hoverLocation))
            }
            if region == nil, focused, selection.length == 0, !suppressedByTyping {
                region = snapshot.scan.region(atCaret: snapshot.utf8(forUTF16: selection.location))
            }
            pendingRegionStart = region?.range.utf8Offset
        }
        // A pending fire already dismissed once resolves to nothing: the
        // dismissal check in fire() runs before any render, so Escape
        // must fall through rather than consume with no effect.
        if pendingRegionStart == dismissedRegionStart { pendingRegionStart = nil }
        // Deadline gone but work pending: an in-flight job can still
        // publish at this generation (completion steals the reply only
        // when generations diverge).
        let liveWork = inFlight.map { $0.generation == generation } ?? false
            || exactInFlight.map { $0.generation == generation } ?? false
        let consumed = visible != nil || pendingRegionStart != nil || liveWork
        guard consumed else { return (false, []) }
        // Suppress the region the pending fire would actually hit — not a
        // stale target left over from an earlier caret position.
        dismissedRegionStart = pendingRegionStart
            ?? target?.region.range.utf8Offset ?? visible?.regionStart
        return (true, retire())
    }

    /// Explicit "exact TeX preview" for the current equation — works in
    /// either renderer mode, once per request.
    public mutating func requestExact(nowMs: UInt64) -> [EquationPreviewCommand] {
        guard let region = target?.region else { return [] }
        explicitExactRegionStart = region.range.utf8Offset
        schedule(at: nowMs)
        return []
    }

    // MARK: Project context

    /// Answers `.resolveIncludes`: `fileID == nil` marks a missing target.
    public mutating func includesResolved(_ results: [(key: MathIncludeKey, fileID: String?)], scans: [String: MathSourceScan], nowMs: UInt64) -> [EquationPreviewCommand] {
        var accepted = false
        for result in results {
            // Only keys we asked for land in resolutions: a stale reply
            // from a previous context must not satisfy this one (B7).
            if pendingResolutions.remove(result.key) != nil {
                resolutions[result.key] = .some(result.fileID)
                accepted = true
            }
        }
        for (id, scan) in scans where id != fileID { externalScans[id] = scan; accepted = true }
        // A wholly stale reply (every key already pruned) changes nothing:
        // no context bump, no cache flush, no refire (B7 r2).
        guard accepted else { return [] }
        contextVersion += 1
        // A changed include (new preamble/definitions, or a removed file)
        // changes exact-render semantics without changing cache keys (C5).
        exactCache.removeAll()
        schedule(at: nowMs)
        return []
    }

    /// A project file changed on disk (or an unsaved buffer was left);
    /// nil drops it. Resolutions are re-checked lazily on the next miss.
    public mutating func externalFileChanged(_ changedFileID: String, scan: MathSourceScan?, nowMs: UInt64) -> [EquationPreviewCommand] {
        guard changedFileID != fileID, externalScans[changedFileID] != scan else { return [] }
        externalScans[changedFileID] = scan
        if scan == nil {
            for (key, value) in resolutions where value == .some(changedFileID) { resolutions[key] = nil }
        }
        contextVersion += 1
        exactCache.removeAll()
        schedule(at: nowMs)
        return []
    }

    // MARK: Renderer results

    public mutating func fastRenderCompleted(key: EquationPreviewKey, outcome: EquationRenderOutcome, nowMs: UInt64) -> [EquationPreviewCommand] {
        guard let active = inFlight, active.key == key else { return [] }
        inFlight = nil
        switch outcome {
        case let .svg(svg): cache.insert(key, svg, cost: svg.utf8.count)
        case let .failed(failure): if failure != .rendererFailed { failures.insert(key, failure, cost: 1) }
        }
        var commands: [EquationPreviewCommand] = []
        if active.generation == generation, var target, target.generation == generation, target.key == key {
            switch outcome {
            case let .svg(svg):
                commands += present(.fast(svg: svg), for: target)
            case let .failed(failure):
                target.failure = failure
                self.target = target
                commands += handleFailure(failure, target: target, nowMs: nowMs)
            }
        }
        if let next = queued {
            queued = nil
            if next.generation == generation, target?.key == next.key {
                if cache.contains(next.key) || failures.contains(next.key) {
                    schedule(at: nowMs)
                } else {
                    inFlight = (next.generation, next.key)
                    commands.append(.render(next))
                }
            }
        }
        return commands
    }

    public mutating func exactRenderCompleted(key: String, outcome: ExactEquationOutcome, nowMs: UInt64) -> [EquationPreviewCommand] {
        guard let active = exactInFlight, active.key == key else { return [] }
        exactInFlight = nil
        if case let .pdf(bytes) = outcome { exactCache.insert(key, bytes, cost: bytes.count) }
        guard active.generation == generation, let target, target.generation == generation, target.exact?.key == key else { return [] }
        switch outcome {
        case let .pdf(bytes): return present(.exact(pdf: bytes), for: target)
        case .failed: return present(.unavailable(.exactFailed), for: target)
        }
    }

    // MARK: Clock

    public mutating func poll(nowMs: UInt64, text: @autoclosure () -> String) -> [EquationPreviewCommand] {
        var commands: [EquationPreviewCommand] = []
        if let deadline = hoverGraceDeadline, deadline <= nowMs {
            hoverGraceDeadline = nil
            if !pointerInPopover {
                // Clear the hover TARGET, not hoverLocation: the pointer may
                // already rest on a new equation C whose region must resolve
                // on this re-fire (C6 r2).
                if target?.trigger == .hover { target = nil }
                schedule(at: nowMs)
            }
        }
        if let deadline = keepDeadline, deadline <= nowMs {
            keepDeadline = nil
            // Still showing an older result for an equation that stayed invalid.
            if target?.failure != nil || target?.region.isRenderable == false, visible != nil {
                visible = nil
                commands.append(.hide)
            }
        }
        if let deadline = fireDeadline, deadline <= nowMs {
            fireDeadline = nil
            commands += fire(nowMs: nowMs, text: text())
        }
        if let deadline = exactDeadline, deadline <= nowMs {
            exactDeadline = nil
            if let target, target.generation == generation, target.failure == .undefinedCommand {
                commands += startExact(for: target)
            }
        }
        return commands
    }

    // MARK: Internals

    private mutating func schedule(at deadline: UInt64) {
        generation += 1
        fireDeadline = deadline
    }

    /// Hide everything and invalidate in-flight publication.
    private mutating func retire() -> [EquationPreviewCommand] {
        generation += 1
        fireDeadline = nil
        keepDeadline = nil
        exactDeadline = nil
        target = nil
        queued = nil
        var commands = cancelExact()
        if visible != nil {
            visible = nil
            commands.append(.hide)
        }
        return commands
    }

    private mutating func cancelExact() -> [EquationPreviewCommand] {
        guard let active = exactInFlight else { return [] }
        exactInFlight = nil
        return [.cancelExact(key: active.key)]
    }

    private mutating func hideNow() -> [EquationPreviewCommand] {
        target = nil
        keepDeadline = nil
        exactDeadline = nil
        var commands = cancelExact()
        if visible != nil {
            visible = nil
            commands.append(.hide)
        }
        return commands
    }

    private mutating func currentSnapshot(_ text: String) -> Snapshot? {
        guard let fileID else { return nil }
        if let snapshot, snapshot.revision == textRevision, snapshot.fileID == fileID { return snapshot }
        var native = text
        native.makeContiguousUTF8()
        let fresh = Snapshot(revision: textRevision, fileID: fileID, text: native, scan: MathSourceScanner.scan(native))
        snapshot = fresh
        return fresh
    }

    private mutating func context(for snapshot: Snapshot) -> (MathProjectContext, [EquationPreviewCommand]) {
        if let cached = projectContext, cached.version == contextVersion, cached.revision == snapshot.revision {
            return (cached.context, [])
        }
        // The active file's scan is passed as an overlay so the whole
        // externalScans map is not cloned on every rebuild (C11).
        let root = rootFileID ?? snapshot.fileID
        let context = MathProjectContext(
            rootFileID: root, activeFileID: snapshot.fileID, activeScan: snapshot.scan,
            scans: externalScans, resolutions: resolutions)
        projectContext = (contextVersion, snapshot.revision, context)
        // A context switch can orphan keys from the previous context: the
        // host may never answer them, and fire() waits on every pending key
        // (B7). Keep only what the fresh context still needs answered.
        pendingResolutions.formIntersection(Set(context.unresolved.map(\.key)))
        let requests = context.unresolved.filter { pendingResolutions.insert($0.key).inserted }
        return (context, requests.isEmpty ? [] : [.resolveIncludes(requests)])
    }

    private mutating func fire(nowMs: UInt64, text: String) -> [EquationPreviewCommand] {
        guard settings.enabled, !composing, rendererVersion != nil || !rendererFailed,
              let snapshot = currentSnapshot(text) else { return hideNow() }

        var region: MathRegion?
        var trigger = EquationPreviewTrigger.caret
        if let hoverLocation {
            region = snapshot.scan.region(atCharacter: snapshot.utf8(forUTF16: hoverLocation))
            trigger = .hover
            // Crossing plain text toward the popover must not drop it at once.
            if region == nil, target?.trigger == .hover, visible != nil, !pointerInPopover {
                if hoverGraceDeadline == nil { hoverGraceDeadline = nowMs + Self.hoverExitGraceMs }
                return []
            }
            // Reaching a math region ends the exit-grace window (C6).
            if region != nil { hoverGraceDeadline = nil }
        }
        // B3 r2: this early return protects only a HOVER-target preview; a
        // caret target keeps refreshing while the pointer rests on the
        // popover.
        if region == nil, pointerInPopover, visible != nil, target?.trigger == .hover { return [] }
        if region == nil, focused, selection.length == 0, !suppressedByTyping {
            region = snapshot.scan.region(atCaret: snapshot.utf8(forUTF16: selection.location))
            trigger = .caret
        }
        guard let region else { return hideNow() }
        let regionStart = region.range.utf8Offset
        if let dismissed = dismissedRegionStart {
            if dismissed == regionStart { return hideNow() }
            dismissedRegionStart = nil
        }
        let anchor = snapshot.utf16(forUTF8: regionStart)..<snapshot.utf16(forUTF8: region.range.endUTF8Offset)
        let sameEquation = visible?.regionStart == regionStart || target?.region.range.utf8Offset == regionStart
        if !sameEquation { keepDeadline = nil }

        guard region.isRenderable else {
            target = Target(generation: generation, region: region, anchor: anchor, trigger: trigger, key: nil,
                            exact: nil, failure: .invalid)
            exactDeadline = nil
            return cancelExact() + keepOrHide(nowMs: nowMs)
        }

        let source = region.renderSource(in: snapshot.text)
        let (projectContext, contextCommands) = context(for: snapshot)
        var commands = contextCommands
        guard source.utf8.count <= Self.maximumSourceBytes else {
            let target = Target(generation: generation, region: region, anchor: anchor, trigger: trigger, key: nil,
                                exact: nil, failure: .tooLarge)
            self.target = target
            return commands + present(.unavailable(.tooLarge), for: target)
        }
        let mathContext = projectContext.context(fileID: snapshot.fileID, utf8Offset: regionStart)
        let key = EquationPreviewKey(
            source: EquationPreviewKey.normalize(source), displayMode: region.displayMode,
            contextKey: mathContext.key, rendererVersion: rendererVersion ?? "", fontSize: appearance.fontSize
        )
        let regionText = region.sourceText(in: snapshot.text)
        let exact = exactProfile.map { profile in
            exactPlan(profile: profile, region: regionText, context: projectContext, snapshot: snapshot, regionStart: regionStart)
        }
        var target = Target(generation: generation, region: region, anchor: anchor, trigger: trigger, key: key,
                            exact: exact, failure: nil)

        if let active = exactInFlight {
            if active.key == exact?.key {
                exactInFlight = (generation, active.key)
            } else {
                commands += cancelExact()
            }
        }
        if explicitExactRegionStart == regionStart {
            self.target = target
            return commands + startExact(for: target)
        }
        exactDeadline = nil
        // Definitions from unresolved includes are still missing: rendering
        // now would flash "unavailable". The host's answer fires again.
        if !pendingResolutions.isEmpty {
            self.target = target
            return commands
        }

        if let svg = cache.value(for: key) {
            self.target = target
            return commands + present(.fast(svg: svg), for: target)
        }
        if let failure = failures.value(for: key) {
            target.failure = failure
            self.target = target
            return commands + handleFailure(failure, target: target, nowMs: nowMs)
        }
        self.target = target
        guard rendererVersion != nil else { return commands } // page still loading: ready re-fires
        let request = EquationRenderRequest(
            generation: generation, key: key, source: source, displayMode: region.displayMode,
            definitions: mathContext.definitions, fontSize: appearance.fontSize
        )
        if let active = inFlight {
            if active.key == key {
                inFlight = (generation, key)
            } else {
                queued = request
            }
        } else {
            inFlight = (generation, key)
            commands.append(.render(request))
        }
        return commands
    }

    private func exactPlan(profile: String, region: String, context: MathProjectContext, snapshot: Snapshot, regionStart: Int) -> ExactPlan {
        let root = context.rootFileID
        let rootScan = root == snapshot.fileID ? snapshot.scan : externalScans[root]
        let preamble = rootScan?.preamble
        let definitions = context.bodyDefinitions(
            fileID: snapshot.fileID, utf8Offset: regionStart,
            rootDocumentBegin: preamble == nil ? nil : rootScan?.documentBeginOffset
        )
        let document = preamble == nil
            ? ExactEquationDocument.make(preamble: nil, preambleDefinitions: definitions, bodyDefinitions: [], region: region)
            : ExactEquationDocument.make(preamble: preamble, preambleDefinitions: [], bodyDefinitions: definitions, region: region)
        let key = MathPreviewHash.hex(MathPreviewHash.combine(MathPreviewHash.combine(MathPreviewHash.basis, profile), document))
        return ExactPlan(key: key, document: document)
    }

    private mutating func startExact(for target: Target) -> [EquationPreviewCommand] {
        guard let exact = target.exact else { return present(.unavailable(.exactUnavailable), for: target) }
        if let pdf = exactCache.value(for: exact.key) { return present(.exact(pdf: pdf), for: target) }
        if let active = exactInFlight, active.key == exact.key {
            exactInFlight = (generation, exact.key)
            return []
        }
        var commands = cancelExact()
        exactInFlight = (generation, exact.key)
        commands.append(.renderExact(ExactEquationRequest(generation: generation, key: exact.key, document: exact.document)))
        return commands
    }

    private mutating func handleFailure(_ failure: EquationRenderFailure, target: Target, nowMs: UInt64) -> [EquationPreviewCommand] {
        switch failure {
        case .undefinedCommand:
            guard settings.renderer == .fastWithTeXFallback else { return present(.unavailable(.unsupported), for: target) }
            guard let exact = target.exact else { return present(.unavailable(.exactUnavailable), for: target) }
            if let pdf = exactCache.value(for: exact.key) { return present(.exact(pdf: pdf), for: target) }
            // TeX runs only once the equation has settled — never per keystroke.
            let commands = keepOrHide(nowMs: nowMs)
            exactDeadline = nowMs + Self.exactSettleMs
            return commands
        case .invalid:
            return keepOrHide(nowMs: nowMs)
        case .tooLarge:
            return present(.unavailable(.tooLarge), for: target)
        case .rendererFailed:
            return hideNow()
        }
    }

    /// An invalid intermediate state: the previous preview of the same
    /// equation stays briefly, anything else hides now. The target (and an
    /// adopted exact job) survive for the next result.
    private mutating func keepOrHide(nowMs: UInt64) -> [EquationPreviewCommand] {
        guard let visible, visible.regionStart == target?.region.range.utf8Offset else {
            keepDeadline = nil
            guard visible != nil else { return [] }
            self.visible = nil
            return [.hide]
        }
        if keepDeadline == nil { keepDeadline = nowMs + Self.keepLastGoodMs }
        return []
    }

    private mutating func present(_ content: EquationPreviewPresentation.Content, for target: Target) -> [EquationPreviewCommand] {
        keepDeadline = nil
        visible = Visible(regionStart: target.region.range.utf8Offset, content: content)
        return [.show(EquationPreviewPresentation(
            generation: target.generation, content: content, anchor: target.anchor,
            anchorUTF8: target.region.range, displayMode: target.region.displayMode,
            placement: settings.placement, trigger: target.trigger
        ))]
    }
}
