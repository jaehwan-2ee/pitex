import PDFKit
import SwiftUI
import SyncTeXCore
import TexDomain
import WebKit

/// Right-hand inspector column: the PDF preview only. The assistant moved
/// into the bottom console so both can stay visible at once.
struct Preview: View {
    @ObservedObject var workspace: WorkspaceModel
    @ObservedObject private var settings: SettingsStore
    /// Observed so a light/dark flip re-renders the Markdown preview when
    /// its theme is "Match app".
    @ObservedObject private var appearance = AppearanceSettings.shared
    /// The Markdown toolbar's Download reaches the live web view through it.
    @State private var markdownExport = MarkdownPDFExport()
    /// True inside the popped-out window — the toolbar's third button then
    /// offers Reattach instead of Detach.
    private let detached: Bool

    init(workspace: WorkspaceModel, detached: Bool = false) {
        self.workspace = workspace
        self.detached = detached
        _settings = ObservedObject(wrappedValue: workspace.settings)
    }

    var body: some View {
        preview
            .frame(minWidth: 260, maxHeight: .infinity, alignment: .top)
            .accessibilityIdentifier("pitex.inspector")
    }

    /// `.md` / `.markdown` documents swap the PDF column for the live
    /// Markdown rendering; everything else keeps the SyncTeX/PDF stack.
    private var activeIsMarkdown: Bool {
        workspace.activeDocumentIsMarkdown
    }

    // MARK: - Preview

    @ViewBuilder
    private var preview: some View {
        if activeIsMarkdown, let url = workspace.activeDocumentURL {
            VStack(spacing: 0) {
                // Same toolbar as the PDF preview: document name + Download.
                HStack(spacing: 8) {
                    Text(verbatim: url.lastPathComponent)
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.secondary)
                    Spacer()
                    Button {
                        markdownExport.save(suggestedName: url.deletingPathExtension().lastPathComponent + ".pdf")
                    } label: {
                        Image(systemName: "arrow.down.circle")
                    }
                    .buttonStyle(.borderless)
                    .help(String(localized: "preview.download"))
                    .accessibilityIdentifier("pitex.preview.markdown.download")
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                Divider()
                MarkdownPreviewView(
                    documentURL: url,
                    text: workspace.documentSnapshot?.text ?? "",
                    fontSize: settings.markdownFontSize,
                    dark: MarkdownLinkPolicy.previewIsDark(
                        theme: settings.markdownTheme,
                        appIsDark: appearance.effectiveDark
                    ),
                    livePreview: settings.markdownLivePreview,
                    syncScroll: settings.markdownSyncScroll,
                    documentClean: workspace.documentSnapshot?.saveState == .clean,
                    sync: workspace.markdownScrollSync,
                    exporter: markdownExport,
                    onBlockedLink: {
                        workspace.agent?.statusMessage = String(localized: "preview.markdown.blocked_link")
                    }
                )
                // A new documentURL rebuilds the web view; text/settings edits
                // flow through updateNSView.
                .id(url)
                .frame(maxHeight: .infinity)
                .accessibilityIdentifier("pitex.preview.markdown")
            }
        } else {
            pdfPreview
        }
    }

    @ViewBuilder
    private var pdfPreview: some View {
        VStack(spacing: 0) {
            syncTeXStatus
            Divider()
            // The retained PDF — last output a build actually published —
            // keeps rendering through building/failed/cancelled states;
            // the view only unmounts when no output exists at all.
            if let retained = workspace.retainedPDF {
                HStack(spacing: 8) {
                    Text(verbatim: pdfDisplayName)
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.secondary)
                    outputKindBadge(retained)
                    Spacer()
                    Button {
                        openPDFExternally()
                    } label: {
                        Image(systemName: "arrow.up.forward.app")
                    }
                    .buttonStyle(.borderless)
                    .disabled(!retained.isEmbeddedPreview && pdfFileURL == nil)
                    .help(String(localized: "preview.open_external"))
                    .accessibilityIdentifier("pitex.pdf.openExternal")
                    Button {
                        downloadPDF(retained.data)
                    } label: {
                        Image(systemName: "arrow.down.circle")
                    }
                    .buttonStyle(.borderless)
                    .help(String(localized: "preview.download"))
                    .accessibilityIdentifier("pitex.pdf.download")
                    Button {
                        detached ? workspace.attachPreview() : workspace.detachPreview()
                    } label: {
                        Image(systemName: detached ? "pip.exit" : "pip.enter")
                    }
                    .buttonStyle(.borderless)
                    .help(String(localized: detached ? "preview.attach" : "preview.detach"))
                    .accessibilityIdentifier(detached ? "pitex.pdf.attach" : "pitex.pdf.detach")
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                Divider()
                if case let .failed(reason) = workspace.buildState {
                    // The last-good PDF stays up but the failure is
                    // still visible — details live in Build Log/Issues.
                    Text(verbatim: reason)
                        .font(.caption)
                        .foregroundStyle(.red)
                        .lineLimit(2)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 4)
                        .accessibilityIdentifier("pitex.preview.buildError")
                    Divider()
                }
                PDFDocumentView(
                    data: retained.data,
                    // Identity is the workspace + resolved source, NOT the
                    // artifact path: a manual main.pdf and a live
                    // .pitex-live/main/main.pdf of the same source must keep
                    // the viewport across the first auto build.
                    target: "\(workspace.projectGeneration.uuidString)/\(retained.sourceTarget)",
                    workspace: workspace,
                    highlightSync: settings.forwardSyncHighlight,
                    onInverseSync: { page, point in
                        Task { await workspace.syncInverse(page: page, point: point) }
                    }
                )
                .frame(maxHeight: .infinity)
                .accessibilityIdentifier("pitex.pdf")
                // The embedded status overlays the PDF instead of sitting in
                // the column above it: a row that is inserted and removed on
                // every typing cycle moved the PDF view by its own height.
                .overlay(alignment: .top) { embeddedStatusOverlay }
            } else {
                embeddedStatusLine
                ContentUnavailableView(
                    "preview.no_pdf",
                    systemImage: "doc.richtext",
                    description: Text("preview.no_pdf_detail")
                )
                .frame(maxHeight: .infinity)
                .accessibilityIdentifier("pitex.preview.empty")
            }
        }
        .frame(maxHeight: .infinity)
        .accessibilityIdentifier("pitex.preview")
    }

    private var syncTeXStatus: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Image(systemName: syncTeXSymbol)
                .foregroundStyle(syncTeXColor)
            VStack(alignment: .leading, spacing: 2) {
                Text("preview.synctex")
                    .font(.caption.weight(.semibold))
                Text(verbatim: syncTeXMessage)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    // One line, always: a longer message that wrapped changed
                    // this row's height and so the PDF view's frame. The full
                    // text stays in the tooltip and is the explicit
                    // accessibility label, so truncation never hides it from
                    // VoiceOver.
                    .lineLimit(1)
                    .truncationMode(.tail)
                    .help(syncTeXMessage)
                    .accessibilityLabel(syncTeXMessage)
            }
            Spacer()
        }
        .padding(10)
        .accessibilityIdentifier("pitex.synctexStatus")
    }

    /// Editing preview vs final compiler output — a preview is never
    /// presented as the project compiler's PDF. Live compiler builds
    /// under `.pitex-live` carry neither label.
    @ViewBuilder
    private func outputKindBadge(_ retained: RetainedPDF) -> some View {
        if retained.isEmbeddedPreview || retained.isFinalOutput {
            let key: LocalizedStringKey = retained.isEmbeddedPreview ? "preview.embedded_label" : "preview.final_label"
            let tint = retained.isEmbeddedPreview ? Color.orange : Color.green
            Text(key)
                .font(.caption2.weight(.semibold))
                .foregroundStyle(tint)
                .padding(.horizontal, 5)
                .padding(.vertical, 1)
                .background(Capsule().strokeBorder(tint))
                .accessibilityIdentifier("pitex.preview.outputKind")
        }
    }

    /// Embedded engine progress (updating/failed/unavailable), separate
    /// from the final build's state.
    @ViewBuilder
    private var embeddedStatusLine: some View {
        if let status = embeddedStatus {
            embeddedStatusText(status)
            Divider()
        }
    }

    /// Over the PDF view, never in the column above it, so no status
    /// changes the PDF view's frame. Non-hit-testing: Cmd-click, scrolling
    /// and selection reach the PDF underneath.
    @ViewBuilder
    private var embeddedStatusOverlay: some View {
        if let status = embeddedStatus {
            VStack(spacing: 0) {
                embeddedStatusText(status)
                Divider()
            }
            .background(.regularMaterial)
            .allowsHitTesting(false)
        }
    }

    private func embeddedStatusText(_ status: (text: String, color: Color)) -> some View {
        Text(verbatim: status.text)
            .font(.caption)
            .foregroundStyle(status.color)
            .lineLimit(2)
            .help(status.text)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
            .accessibilityIdentifier("pitex.preview.embeddedStatus")
    }

    private var embeddedStatus: (text: String, color: Color)? {
        func joined(_ key: String.LocalizationValue, _ detail: String) -> String {
            let title = String(localized: key)
            return detail.isEmpty ? title : "\(title): \(detail)"
        }
        switch workspace.embeddedPreviewStatus {
        case .off, .current: return nil
        case .updating: return (String(localized: "preview.embedded_updating"), .secondary)
        case let .errors(first): return first.isEmpty ? nil : (first, .orange)
        case let .warnings(message): return (joined("preview.embedded_warnings", message), .orange)
        case let .failed(message): return (joined("preview.embedded_failed", message), .red)
        case let .unavailable(reason): return (joined("preview.embedded_unavailable", reason), .orange)
        }
    }

    /// Display name of the rendered PDF — the artifact's basename even
    /// when the real file hides under `.pitex-live`.
    private var pdfDisplayName: String {
        guard let path = workspace.retainedPDF?.artifactPath ?? workspace.latestBuiltPDFName else {
            return "document.pdf"
        }
        return (path as NSString).lastPathComponent
    }

    /// Saved/opened copies of an editing preview keep its label in the
    /// file name so they are never mistaken for the final PDF.
    private var exportName: String {
        guard workspace.retainedPDF?.isEmbeddedPreview == true else { return pdfDisplayName }
        let stem = (pdfDisplayName as NSString).deletingPathExtension
        return "\(stem) (\(String(localized: "preview.embedded_label"))).pdf"
    }

    /// On-disk URL of the last built PDF — the SyncTeX binding carries it
    /// once a build has succeeded; before binding it derives from the
    /// retained artifact path (which may live under `.pitex-live`).
    /// Editing previews have none: their session artifacts are released
    /// as newer ones arrive.
    private var pdfFileURL: URL? {
        guard let retained = workspace.retainedPDF, !retained.isEmbeddedPreview else { return nil }
        if let url = workspace.syncTeXBinding?.pdfURL { return url }
        guard let root = workspace.projectURL else { return nil }
        return retained.fileURL(projectRoot: root)
    }

    /// Opens the built PDF in the system default PDF viewer (Preview.app);
    /// an editing preview opens as a labeled copy of the bytes on screen.
    private func openPDFExternally() {
        if let retained = workspace.retainedPDF, retained.isEmbeddedPreview {
            guard let url = try? WorkspaceModel.exportEmbeddedPreviewCopy(retained.data, named: exportName) else { return }
            NSWorkspace.shared.open(url)
            return
        }
        guard let url = pdfFileURL else { return }
        NSWorkspace.shared.open(url)
    }

    /// "Download": saves a copy of the rendered PDF via a save panel.
    private func downloadPDF(_ data: Data) {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = exportName
        panel.allowedContentTypes = [.pdf]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        try? data.write(to: url)
    }

    private var syncTeXSymbol: String {
        switch workspace.syncTeXState {
        case .current: "checkmark.circle"
        case .stale: "clock.badge.exclamationmark"
        case .ambiguous: "arrow.triangle.branch"
        case .unavailable: "slash.circle"
        }
    }

    private var syncTeXColor: Color {
        switch workspace.syncTeXState {
        case .current: .green
        case .stale, .ambiguous: .orange
        case .unavailable: .secondary
        }
    }

    private var syncTeXMessage: String {
        switch workspace.syncTeXState {
        case .current: "Source and PDF revisions match."
        case let .stale(reason), let .ambiguous(reason), let .unavailable(reason): reason
        }
    }
}

/// The macOS PDF viewer is PDFKit's PDFView: main's wrapper plus a keep-alive
/// of the outgoing document (`retiredDocument`) and a swap cover that holds
/// the outgoing pages' image over the content area while the new document
/// draws. Apart from those two additions it is the manifested source snapshot
/// (the control arm of the D-v7 / document-swap trials,
/// sha256 9b0446fadb6910fa149360278c567ff989211ea187396446bf9a663d5b737c0e) minus its
/// two per-swap NSLog lines, which logged file paths.
private struct PDFDocumentView: NSViewRepresentable {
    let data: Data
    /// Workspace + source identity of these bytes — a different build
    /// source or a reopened workspace resets the view instead of
    /// preserving a page index that belonged to another document.
    let target: String
    /// Only this workspace's forward-sync highlights reach this view — every
    /// window has its own PDF, and the notification is app-wide.
    let workspace: AnyObject
    var highlightSync = false
    var onInverseSync: (Int, SyncTeXCore.PDFPoint) -> Void = { _, _ in }

    func makeNSView(context: Context) -> PDFView {
        let view = PDFView()
        view.autoScales = true
        view.displayMode = .singlePageContinuous
        view.displaysPageBreaks = true
        view.document = PDFDocument(data: data)

        // Cmd-click on a PDF location triggers inverse SyncTeX — the
        // reference editor's Ctrl-click equivalent. A local event monitor is
        // used instead of an NSClickGestureRecognizer: recognizers lose
        // clicks to PDFView's own tracking, while the monitor sees the raw
        // event first and can swallow it before selection/link behavior.
        context.coordinator.syncMonitor = NSEvent.addLocalMonitorForEvents(
            matching: [.leftMouseDown, .rightMouseDown, .otherMouseDown, .scrollWheel, .magnify, .smartMagnify, .keyDown]
        ) { [weak coordinator = context.coordinator] event in
            let consumed = MainActor.assumeIsolated {
                // An interaction takes the swap cover away first, so a Cmd-click maps on the document beneath.
                coordinator?.removeCoverOnInteraction(event)
                return event.type == .leftMouseDown ? (coordinator?.handleSyncClick(event) ?? false) : false
            }
            return consumed ? nil : event
        }

        context.coordinator.pdfView = view
        context.coordinator.renderedData = data
        context.coordinator.renderedTarget = target
        context.coordinator.onInverseSync = onInverseSync
        context.coordinator.highlightSync = highlightSync

        NotificationCenter.default.addObserver(
            context.coordinator,
            selector: #selector(Coordinator.highlightRequested(_:)),
            name: .syncTeXHighlightRequested,
            object: workspace
        )
        return view
    }

    func updateNSView(_ view: PDFView, context: Context) {
        context.coordinator.onInverseSync = onInverseSync
        context.coordinator.highlightSync = highlightSync
        let sameTarget = context.coordinator.renderedTarget == target
        // Fast path: Data is a value type over shared storage, so the same
        // buffer compares equal by base address without a multi-MB memcmp
        // on every SwiftUI pass. Distinct storage falls back to ==.
        let unchanged = context.coordinator.renderedData.count == data.count
            && context.coordinator.renderedData.withUnsafeBytes { rendered in
                data.withUnsafeBytes { incoming in
                    rendered.baseAddress == incoming.baseAddress
                }
            }
        // Rebuilding `view.document` resets the PDF to its first page, so
        // equal bytes — same storage or a byte-compare hit — must return
        // early; only genuinely new build output reloads the view. The
        // identity must also match: same bytes under a DIFFERENT source
        // still swap so renderedTarget advances and the position resets.
        if sameTarget && (unchanged || context.coordinator.renderedData == data) { return }
        // Capture BEFORE the swap: index and point come from the SAME
        // currentDestination — in continuous mode currentPage can differ
        // from currentDestination.page near page boundaries, and mixing
        // them jumps. index(for:) yields NSNotFound (not nil) for a page
        // outside the old document — normalize it so the currentPage
        // fallback can apply; a nil point never pairs a fallback index
        // with a destination that resolved to no page.
        let destination = sameTarget ? view.currentDestination : nil
        let destinationIndex = destination?.page
            .flatMap { view.document?.index(for: $0) }
            .flatMap { $0 == NSNotFound ? nil : $0 }
        let oldIndex = sameTarget
            ? destinationIndex ?? view.currentPage
                .flatMap { view.document?.index(for: $0) }
                .flatMap { $0 == NSNotFound ? nil : $0 }
            : nil
        let point = destinationIndex != nil ? destination?.point : nil
        let autoScales = view.autoScales
        let scaleFactor = view.scaleFactor
        context.coordinator.clearSyncHighlight()
        context.coordinator.renderedData = data
        context.coordinator.renderedTarget = target
        // The swap cover: a still image of the outgoing pages over the content area, taken before the
        // document is replaced (after the highlight is cleared, so it never shows one the swap removes).
        context.coordinator.coverOutgoing(view, sameTarget: sameTarget)
        // PDFView renders pages asynchronously and can still be decoding
        // the outgoing document when the swap lands — its PDFPage backrefs
        // are weak, so a dropped document leaves "drawing a PDFPage when
        // its PDFDocument is nil" warnings. Keep exactly one retired
        // document alive until the next swap or dismantle.
        context.coordinator.retiredDocument = view.document
        let incoming = PDFDocument(data: data)
        if context.coordinator.hasCover {
            // The draw watch is registered BEFORE the document goes in: PDFKit draws off the main thread and may finish a page
            // before this turn ends, and a report for a document nobody waits for is dropped.
            incoming?.delegate = context.coordinator.drawDelegate
            context.coordinator.awaitDraw(incoming)
        }
        view.document = incoming
        guard sameTarget, let document = view.document, document.pageCount > 0 else { return }
        view.autoScales = autoScales
        if !autoScales { view.scaleFactor = scaleFactor }
        if let oldIndex {
            // The new document may be shorter — clamp the index, not a page.
            let clamped = min(max(oldIndex, 0), document.pageCount - 1)
            if let page = document.page(at: clamped) {
                if let point {
                    view.go(to: PDFDestination(page: page, at: point))
                } else {
                    // No destination was set — keep the page itself.
                    view.go(to: page)
                }
            }
        }
    }

    static func dismantleNSView(_ nsView: PDFView, coordinator: Coordinator) {
        coordinator.clearSyncHighlight()
        coordinator.removeCover()
        NotificationCenter.default.removeObserver(coordinator)
        if let monitor = coordinator.syncMonitor {
            NSEvent.removeMonitor(monitor)
        }
        // Release both documents (and every PDFPage held through
        // destinations and annotations) while the view is still live —
        // PDFKit warns "drawing a PDFPage when its PDFDocument is nil"
        // when pages outlive their document during removal.
        coordinator.retiredDocument = nil
        nsView.document = nil
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    @MainActor
    final class Coordinator: NSObject {
        weak var pdfView: PDFView?
        var renderedData = Data()
        /// Workspace + source identity of the rendered document —
        /// distinguishes a rebuild of the same target (preserve
        /// position) from a target or workspace switch (reset).
        var renderedTarget = ""
        var syncMonitor: Any?
        /// The previously displayed document, kept alive one swap: PDFView
        /// decodes pages on background threads with weak document
        /// backrefs, so releasing it at swap time orphans in-flight pages.
        var retiredDocument: PDFDocument?
        var onInverseSync: (Int, SyncTeXCore.PDFPoint) -> Void = { _, _ in }
        var highlightSync = false {
            didSet { if !highlightSync { clearSyncHighlight() } }
        }
        private var syncHighlight: PDFAnnotation?
        private var highlightTask: Task<Void, Never>?

        func clearSyncHighlight() {
            highlightTask?.cancel()
            highlightTask = nil
            if let syncHighlight { syncHighlight.page?.removeAnnotation(syncHighlight) }
            syncHighlight = nil
        }

        /// Cmd+click inside the PDF fires inverse SyncTeX and reports the
        /// event as consumed so text selection and link following stay out of
        /// the way; every other click passes through to the PDF view.
        func handleSyncClick(_ event: NSEvent) -> Bool {
            guard event.modifierFlags.contains(.command),
                  let view = pdfView,
                  event.window === view.window
            else { return false }
            let location = view.convert(event.locationInWindow, from: nil)
            guard view.bounds.contains(location),
                  let page = view.page(for: location, nearest: true)
            else { return false }
            // PDFKit page space is bottom-left origin; `synctex edit` wants
            // top-left origin points — the same flip the forward highlight
            // performs in reverse.
            let pagePoint = view.convert(location, to: page)
            let mediaBox = page.bounds(for: .mediaBox)
            guard let point = try? SyncTeXCore.PDFPoint(
                x: Double(pagePoint.x - mediaBox.origin.x),
                y: Double(mediaBox.maxY - pagePoint.y)
            ) else { return false }
            // `index(for:)` is 0-based; `synctex edit` numbers pages from 1.
            onInverseSync((view.document?.index(for: page) ?? 0) + 1, point)
            return true
        }

        @objc func highlightRequested(_ notification: Notification) {
            guard let view = pdfView,
                  let info = notification.userInfo,
                  let pageNumber = info["page"] as? Int,
                  let document = view.document,
                  pageNumber > 0, pageNumber <= document.pageCount,
                  let page = document.page(at: pageNumber - 1)
            else { return }
            // SyncTeX's v is the box's bottom edge, measured from the page
            // top. Its height extends upward in PDFKit's page coordinates.
            let x = info["x"] as? Double ?? 0
            let v = info["y"] as? Double ?? 0
            let width = info["width"] as? Double ?? 0
            let height = info["height"] as? Double ?? 0
            let mediaBox = page.bounds(for: .mediaBox)
            let pdfY = mediaBox.maxY - v
            let bounds = NSRect(
                x: mediaBox.origin.x + x,
                y: pdfY,
                width: max(width, 4),
                height: max(height, 4)
            )
            clearSyncHighlight()
            view.go(to: bounds.insetBy(dx: -8, dy: -8), on: page)
            guard highlightSync else { return }
            let marker = PDFAnnotation(bounds: bounds, forType: .highlight, withProperties: nil)
            marker.color = NSColor.systemYellow.withAlphaComponent(0.35)
            marker.shouldPrint = false
            page.addAnnotation(marker)
            syncHighlight = marker
            highlightTask = Task { @MainActor [weak self] in
                do { try await Task.sleep(for: .seconds(1.5)) } catch { return }
                self?.clearSyncHighlight()
            }
        }

        // ─── Swap cover ───────────────────────────────────────────────────────
        // PDFView swaps the page tiles of a new document in as they finish, and for a
        // while the content area shows nothing. The cover holds the outgoing pages'
        // image over the content area (never over the scrollers) until PDFKit has drawn
        // a visible page of the new document, or 1.2 s have passed, or the user interacts.
        private var cover: SwapCoverView?
        private var awaited: ObjectIdentifier?
        private var drawSeen = false
        private var coverGeneration = 0
        private var drawGeneration = 0
        private var frameObserver: NSObjectProtocol?
        /// `PDFDocument.delegate` is weak: the coordinator keeps the one delegate alive.
        let drawDelegate = SwapDrawDelegate()
        var hasCover: Bool { cover != nil }

        /// Called in the swap turn, before the document is replaced. A swap while covered keeps the cover
        /// (the view beneath may be undrawn, so no new snapshot) unless the target changed.
        func coverOutgoing(_ view: PDFView, sameTarget: Bool) {
            if cover != nil {
                if !sameTarget { removeCover() }
                return
            }
            guard sameTarget, !view.isHidden, let window = view.window,
                  let document = view.document, document.pageCount > 0 else { return }
            let pages = view.visiblePages
            guard !pages.isEmpty, pages.allSatisfy({ $0.rotation == 0 }) else { return }
            let clip = Self.clipRect(of: view)
            guard clip.width > 1, clip.height > 1 else { return }
            let scale = window.backingScaleFactor
            let rects = pages.map { view.convert($0.bounds(for: .cropBox), from: $0) }
            // c31:sample
            let sampled = Self.underPageColor(of: view, clip: clip, pageRects: rects)
            // c31:sampled
            guard let image = Self.renderOutgoing(view, pages: pages, rects: rects, clip: clip, underPage: sampled.0, scale: scale)
            else { return }
            let swapCover = SwapCoverView(frame: clip, image: image, scale: scale)
            view.addSubview(swapCover)
            cover = swapCover
            // c31:install
            coverGeneration += 1
            let generation = coverGeneration
            // The cap: whatever happens, the cover is gone 1.2 s after it was installed.
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { [weak self] in
                MainActor.assumeIsolated {
                    if let self, self.coverGeneration == generation { self.removeCover() }
                }
            }
            frameObserver = NotificationCenter.default.addObserver(
                forName: NSView.frameDidChangeNotification, object: view, queue: .main
            ) { [weak self] _ in
                MainActor.assumeIsolated { self?.removeCover() }
            }
        }

        /// Called before the incoming document is set: the cover waits for PDFKit to draw a visible page of it. A later
        /// swap while covered retargets the wait to the newest document; an unreadable or empty document removes the cover.
        func awaitDraw(_ incoming: PDFDocument?) {
            guard cover != nil else { return }
            drawGeneration += 1
            drawSeen = false
            if let previous = awaited { SwapDrawRegistry.shared.forget(previous) }
            awaited = nil
            guard let document = incoming, document.pageCount > 0 else {
                removeCover()
                return
            }
            let identity = ObjectIdentifier(document)
            awaited = identity
            SwapDrawRegistry.shared.watch(identity) { [weak self] page in
                DispatchQueue.main.async {
                    MainActor.assumeIsolated { self?.pageDrawn(document: identity, page: page) }
                }
            }
        }

        private func pageDrawn(document identity: ObjectIdentifier, page: ObjectIdentifier) {
            guard cover != nil, !drawSeen, awaited == identity, let view = pdfView, let document = view.document,
                  ObjectIdentifier(document) == identity,
                  view.visiblePages.contains(where: { ObjectIdentifier($0) == page }) else { return }
            drawSeen = true
            // c31:signal
            // A fixed margin lets the compositor present the frame the draw belongs to (a heuristic, bounded by the cap).
            let generation = drawGeneration
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self] in
                MainActor.assumeIsolated {
                    if let self, self.drawGeneration == generation { self.removeCover() }
                }
            }
        }

        func removeCover() {
            guard let swapCover = cover else { return }
            // c31:remove
            swapCover.removeFromSuperview()
            cover = nil
            coverGeneration += 1
            drawGeneration += 1
            if let awaited { SwapDrawRegistry.shared.forget(awaited) }
            awaited = nil
            if let frameObserver { NotificationCenter.default.removeObserver(frameObserver) }
            frameObserver = nil
        }

        /// A click, scroll, magnify or (with the PDF view focused) key press inside the PDF view removes the cover.
        /// Typing in the editor does not: typing is what causes the swaps.
        func removeCoverOnInteraction(_ event: NSEvent) {
            guard cover != nil, let view = pdfView, let window = view.window, event.window === window else { return }
            if event.type == .keyDown {
                guard let responder = window.firstResponder as? NSView,
                      responder === view || responder.isDescendant(of: view) else { return }
            } else {
                guard view.bounds.contains(view.convert(event.locationInWindow, from: nil)) else { return }
            }
            removeCover()
        }

        /// The scroll view's content area in the PDF view's coordinates (the legacy scrollers lie outside it).
        private static func clipRect(of view: PDFView) -> NSRect {
            guard let clipView = view.documentView?.enclosingScrollView?.contentView, let parent = clipView.superview
            else { return view.bounds }
            return view.convert(clipView.frame, from: parent).intersection(view.bounds)
        }

        /// What PDFKit shows where no page is: a point on the left edge of the content area that no page covers
        /// (top, bottom, then the gaps between pages; never the right or bottom band where an overlay scroller can
        /// appear), read with `cacheDisplay`. With no such point the pages fill the area and the view's background
        /// is used (it cannot be seen).
        private static func underPageColor(of view: PDFView, clip: NSRect, pageRects: [NSRect]) -> (CGColor, NSPoint?) {
            var background = view.backgroundColor.cgColor
            view.effectiveAppearance.performAsCurrentDrawingAppearance { background = view.backgroundColor.cgColor }
            let x = clip.minX + 2
            var candidates = [NSPoint(x: x, y: clip.maxY - 2), NSPoint(x: x, y: clip.minY + 2)]
            let rows = pageRects.map { ($0.minY, $0.maxY) }.sorted { $0.0 < $1.0 }
            for (below, above) in zip(rows, rows.dropFirst()) where above.0 > below.1 {
                candidates.append(NSPoint(x: x, y: (below.1 + above.0) / 2))
            }
            for point in candidates where clip.contains(point) && !pageRects.contains(where: { $0.insetBy(dx: -1, dy: -1).contains(point) }) {
                let pixel = NSRect(x: point.x, y: point.y, width: 1, height: 1)
                guard let rep = view.bitmapImageRepForCachingDisplay(in: pixel) else { continue }
                view.cacheDisplay(in: pixel, to: rep)
                if let color = rep.colorAt(x: 0, y: 0)?.cgColor { return (color, point) }
            }
            return (background, nil)
        }

        /// The outgoing view's visible pages, drawn with `PDFPage.draw` (a `cacheDisplay` of a PDFView carries the page
        /// background but not the page content), cropped to the content area. `CGImage.cropping` may keep the parent
        /// bitmap alive, so the full-view bitmap (at most ~30 MB) is held until the cover is removed.
        private static func renderOutgoing(
            _ view: PDFView, pages: [PDFPage], rects: [NSRect], clip: NSRect, underPage: CGColor, scale: CGFloat
        ) -> CGImage? {
            let size = view.bounds.size
            let width = Int((size.width * scale).rounded()), height = Int((size.height * scale).rounded())
            guard width > 0, height > 0,
                  let rep = NSBitmapImageRep(
                      bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height, bitsPerSample: 8, samplesPerPixel: 4,
                      hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
                  let graphics = NSGraphicsContext(bitmapImageRep: rep) else { return nil }
            let context = graphics.cgContext
            context.scaleBy(x: scale, y: scale)
            context.setFillColor(underPage)
            context.fill(CGRect(origin: .zero, size: size))
            for (page, rect) in zip(pages, rects) {
                let crop = page.bounds(for: .cropBox)
                var target = rect
                if view.isFlipped { target.origin.y = size.height - target.maxY }   // the bitmap context is bottom-left origin
                guard target.width > 0, target.height > 0, crop.width > 0, crop.height > 0 else { return nil }
                context.saveGState()
                context.setFillColor(CGColor(gray: 1, alpha: 1))
                context.fill(target)
                context.translateBy(x: target.minX, y: target.minY)
                context.scaleBy(x: target.width / crop.width, y: target.height / crop.height)
                context.translateBy(x: -crop.minX, y: -crop.minY)
                page.draw(with: .cropBox, to: context)
                context.restoreGState()
            }
            guard let full = rep.cgImage else { return nil }
            // Image rows run top-down; the view's coordinates run bottom-up unless it is flipped.
            let top = view.isFlipped ? clip.minY : size.height - clip.maxY
            let pixels = CGRect(x: clip.minX * scale, y: top * scale, width: clip.width * scale, height: clip.height * scale).integral
            return full.cropping(to: pixels)
        }
    }
}

/// The swap cover: one still image laid over the PDF view's content area. It never takes a click and is not an
/// accessibility element.
private final class SwapCoverView: NSView {
    init(frame: NSRect, image: CGImage, scale: CGFloat) {
        super.init(frame: frame)
        wantsLayer = true
        layer?.contents = image
        layer?.contentsGravity = .resize
        layer?.contentsScale = scale
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { nil }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override func isAccessibilityElement() -> Bool { false }
}

/// Tells the swap cover when PDFKit has drawn a page of the document it waits for. `PDFPage.draw` runs off the main
/// thread, so the page only records two object identities under a lock; the main actor does everything else.
private final class SwapDrawRegistry: @unchecked Sendable {
    static let shared = SwapDrawRegistry()
    private let lock = NSLock()
    private var sinks: [ObjectIdentifier: @Sendable (ObjectIdentifier) -> Void] = [:]

    func watch(_ document: ObjectIdentifier, _ sink: @escaping @Sendable (ObjectIdentifier) -> Void) {
        lock.lock()
        sinks[document] = sink
        lock.unlock()
    }

    func forget(_ document: ObjectIdentifier) {
        lock.lock()
        sinks[document] = nil
        lock.unlock()
    }

    func report(document: ObjectIdentifier, page: ObjectIdentifier) {
        lock.lock()
        let sink = sinks[document]
        lock.unlock()
        sink?(page)
    }
}

private final class SwapDrawPage: PDFPage {
    override func draw(with box: PDFDisplayBox, to context: CGContext) {
        super.draw(with: box, to: context)
        if let document { SwapDrawRegistry.shared.report(document: ObjectIdentifier(document), page: ObjectIdentifier(self)) }
    }
}

private final class SwapDrawDelegate: NSObject, PDFDocumentDelegate {
    func classForPage() -> AnyClass { SwapDrawPage.self }
}

extension Notification.Name {
    static let syncTeXHighlightRequested = Notification.Name("pitex.synctex.highlight")
}

// ─── Markdown preview ────────────────────────────────────────────────────────

/// Editor↔preview scroll-sync channel shared by `EditorContainerView`
/// (reports the editor's top line, scrolls on preview reports) and the
/// Markdown web view. The JS side suppresses its own echo after
/// `pitexScrollToLine`; `editorQuietUntil` mutes the reverse echo.
@MainActor
final class MarkdownScrollSync {
    /// Editor → preview: called with the 0-based top visible source line.
    var onEditorTopLine: ((Int) -> Void)?
    /// Preview → editor: scrolls the editor so `line` sits at the top.
    var scrollEditorToLine: ((Int) -> Void)?
    private var editorQuietUntil = Date.distantPast

    func editorScrolled(to line: Int) {
        guard Date() >= editorQuietUntil else { return }
        onEditorTopLine?(line)
    }

    func previewScrolled(to line: Int) {
        editorQuietUntil = Date().addingTimeInterval(0.12)
        scrollEditorToLine?(line)
    }
}

/// WKWebView host for the shared `markdown-preview.html` renderer — one
/// load at mount, then full-document `pitexRender` pushes. The page CSP
/// (sha256 script hash, no unsafe-inline) plus the strict navigation
/// policy below keep `html: true` raw HTML inert: injected scripts never
/// run, and only the initial load is allowed to navigate.
private struct MarkdownPreviewView: NSViewRepresentable {
    let documentURL: URL
    let text: String
    var fontSize: Double
    var dark: Bool
    var livePreview: Bool
    var syncScroll: Bool
    var documentClean: Bool
    var sync: MarkdownScrollSync
    var exporter: MarkdownPDFExport
    var onBlockedLink: () -> Void

    func makeNSView(context: Context) -> WKWebView {
        let content = WKUserContentController()
        // The content controller retains its handlers — proxy weakly.
        content.add(WeakScriptMessageHandler(context.coordinator), name: "pitexScroll")
        let configuration = WKWebViewConfiguration()
        configuration.userContentController = content
        let view = WKWebView(frame: .zero, configuration: configuration)
        view.navigationDelegate = context.coordinator
        context.coordinator.webView = view
        exporter.webView = view
        context.coordinator.sync = sync
        if let page = Bundle.main.url(forResource: "markdown-preview", withExtension: "html") {
            // The page lives inside the bundle, so the granted read scope
            // must cover both it and the document — `/` does, and the app is
            // unsandboxed anyway. The CSP hash still pins scripts to our
            // bundle; images/relative links (`../figures/x.png`) resolve
            // anywhere on disk, link clicks route through the policy.
            view.loadFileURL(page, allowingReadAccessTo: URL(fileURLWithPath: "/"))
        }
        sync.onEditorTopLine = { [weak coordinator = context.coordinator] line in
            coordinator?.scrollPreview(to: line)
        }
        return view
    }

    func updateNSView(_ view: WKWebView, context: Context) {
        context.coordinator.onBlockedLink = onBlockedLink
        context.coordinator.syncScroll = syncScroll
        // A document switch renders immediately; edits debounce. With live
        // preview off only a clean (saved) document or a settings flip
        // (theme/font size — the last two key parts) re-renders.
        let switched = context.coordinator.renderedURL != documentURL
        let settingsChanged = context.coordinator.renderedFontSize != fontSize
            || context.coordinator.renderedDark != dark
        guard switched || documentClean || livePreview || settingsChanged else { return }
        // Unrelated workspace publishes reach this point with identical
        // content — don't re-encode the whole document when the last render
        // already covered it and no debounced render is queued.
        if context.coordinator.renderTask == nil,
           context.coordinator.renderedText == text,
           context.coordinator.renderedURL == documentURL,
           context.coordinator.renderedFontSize == fontSize,
           context.coordinator.renderedDark == dark {
            return
        }
        context.coordinator.scheduleRender(
            text: text, url: documentURL, fontSize: fontSize, dark: dark,
            immediate: switched || settingsChanged
        )
    }

    static func dismantleNSView(_ view: WKWebView, coordinator: Coordinator) {
        coordinator.renderTask?.cancel()
        coordinator.sync?.onEditorTopLine = nil
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    @MainActor
    final class Coordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
        weak var webView: WKWebView?
        var sync: MarkdownScrollSync?
        var onBlockedLink: (() -> Void)?
        var syncScroll = true
        var renderedURL: URL?
        /// Settings half of the render key — a flip re-renders even while
        /// live preview is off.
        var renderedFontSize: Double?
        var renderedDark: Bool?
        /// Content half of the last completed render — with the other
        /// rendered* fields it lets updateNSView skip no-op updates.
        var renderedText: String?
        var renderTask: Task<Void, Never>?
        private var loaded = false
        private var queuedScript: String?

        func scheduleRender(text: String, url: URL, fontSize: Double, dark: Bool, immediate: Bool) {
            renderTask?.cancel()
            renderTask = Task { @MainActor [weak self] in
                if !immediate {
                    try? await Task.sleep(for: .milliseconds(150))
                    guard !Task.isCancelled else { return }
                }
                self?.render(text: text, url: url, fontSize: fontSize, dark: dark)
                // A cancelled task left renderTask pointing at its
                // replacement — only clear it when that didn't happen.
                if !Task.isCancelled { self?.renderTask = nil }
            }
        }

        func render(text: String, url: URL, fontSize: Double, dark: Bool) {
            renderedURL = url
            renderedFontSize = fontSize
            renderedDark = dark
            renderedText = text
            // The document's directory as a file:// base so relative image
            // and link URLs resolve before the navigation policy sees them.
            let base = url.deletingLastPathComponent().absoluteString
            let payload: [String: Any] = [
                "text": text,
                "baseHref": base,
                "fontSize": fontSize,
                "dark": dark,
            ]
            guard let data = try? JSONSerialization.data(withJSONObject: payload),
                  let json = String(data: data, encoding: .utf8) else { return }
            let script = "pitexRender(\(json))"
            guard loaded else {
                queuedScript = script
                return
            }
            webView?.evaluateJavaScript(script, completionHandler: nil)
        }

        func scrollPreview(to line: Int) {
            guard syncScroll, loaded else { return }
            webView?.evaluateJavaScript("pitexScrollToLine(\(line))", completionHandler: nil)
        }

        // MARK: WKScriptMessageHandler — the page reports a user scroll.

        func userContentController(
            _ controller: WKUserContentController,
            didReceive message: WKScriptMessage
        ) {
            guard syncScroll, let line = (message.body as? NSNumber)?.intValue else { return }
            sync?.previewScrolled(to: line)
        }

        // MARK: WKNavigationDelegate — only the initial load navigates.

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationAction: WKNavigationAction
        ) async -> WKNavigationActionPolicy {
            // The bundled page is the only navigation before `didFinish`:
            // Markdown content is injected afterwards, so nothing it contains
            // can navigate yet. Comparing the exact URL was fragile.
            if !loaded, navigationAction.request.url?.isFileURL == true {
                return .allow
            }
            if navigationAction.navigationType == .linkActivated, let url = navigationAction.request.url {
                switch MarkdownLinkPolicy.action(for: url) {
                case .openExternal(let url), .openFile(let url):
                    NSWorkspace.shared.open(url)
                case .refuse:
                    onBlockedLink?()
                case .ignore:
                    break
                }
            }
            return .cancel
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            loaded = true
            if let script = queuedScript {
                queuedScript = nil
                webView.evaluateJavaScript(script, completionHandler: nil)
            }
        }
    }
}

/// Download for the Markdown preview: prints the live web view to a
/// paginated PDF. The dark palette is screen-only CSS, so the file always
/// comes out light.
@MainActor
final class MarkdownPDFExport {
    weak var webView: WKWebView?

    func save(suggestedName: String) {
        guard let webView, let window = webView.window else { return }
        let panel = NSSavePanel()
        panel.nameFieldStringValue = suggestedName
        panel.allowedContentTypes = [.pdf]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        let info = NSPrintInfo(dictionary: [.jobSavingURL: url])
        info.jobDisposition = .save
        info.horizontalPagination = .fit
        info.verticalPagination = .automatic
        let operation = webView.printOperation(with: info)
        operation.showsPrintPanel = false
        operation.showsProgressPanel = false
        // WKWebView's print view starts with a zero frame, which prints
        // blank pages; run it attached to the window (not run()).
        operation.view?.frame = webView.bounds
        operation.runModal(for: window, delegate: nil, didRun: nil, contextInfo: nil)
    }
}

/// `WKScriptMessageHandler` retains its target — bounce through a weak box
/// so the coordinator can die with the view.
private final class WeakScriptMessageHandler: NSObject, WKScriptMessageHandler {
    weak var target: WKScriptMessageHandler?
    init(_ target: WKScriptMessageHandler) { self.target = target }
    func userContentController(
        _ controller: WKUserContentController,
        didReceive message: WKScriptMessage
    ) {
        target?.userContentController(controller, didReceive: message)
    }
}
