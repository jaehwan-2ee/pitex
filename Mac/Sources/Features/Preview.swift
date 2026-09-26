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

    init(workspace: WorkspaceModel) {
        self.workspace = workspace
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
                    Spacer()
                    Button {
                        openPDFExternally()
                    } label: {
                        Image(systemName: "arrow.up.forward.app")
                    }
                    .buttonStyle(.borderless)
                    .disabled(pdfFileURL == nil)
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
            } else {
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
            }
            Spacer()
        }
        .padding(10)
        .accessibilityIdentifier("pitex.synctexStatus")
    }

    /// Display name of the rendered PDF — the artifact's basename even
    /// when the real file hides under `.pitex-live`.
    private var pdfDisplayName: String {
        guard let path = workspace.retainedPDF?.artifactPath ?? workspace.latestBuiltPDFName else {
            return "document.pdf"
        }
        return (path as NSString).lastPathComponent
    }

    /// On-disk URL of the last built PDF — the SyncTeX binding carries it
    /// once a build has succeeded; before binding it derives from the
    /// retained artifact path (which may live under `.pitex-live`).
    private var pdfFileURL: URL? {
        if let url = workspace.syncTeXBinding?.pdfURL { return url }
        guard let root = workspace.projectURL,
              let artifact = workspace.retainedPDF?.artifactPath else { return nil }
        return root.appendingPathComponent(artifact)
    }

    /// Opens the built PDF in the system default PDF viewer (Preview.app).
    private func openPDFExternally() {
        guard let url = pdfFileURL else { return }
        NSWorkspace.shared.open(url)
    }

    /// "Download": saves a copy of the rendered PDF via a save panel.
    private func downloadPDF(_ data: Data) {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = pdfDisplayName
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
            matching: .leftMouseDown
        ) { [weak coordinator = context.coordinator] event in
            let consumed = MainActor.assumeIsolated {
                coordinator?.handleSyncClick(event) ?? false
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
        // them jumps. Index lookups run on the OLD document; asking the
        // new one for the old page object yields NSNotFound.
        let destination = sameTarget ? view.currentDestination : nil
        let oldIndex = sameTarget
            ? destination.flatMap { view.document?.index(for: $0.page) }
                ?? view.currentPage.flatMap { view.document?.index(for: $0) }
            : nil
        let point = destination?.point
        let autoScales = view.autoScales
        let scaleFactor = view.scaleFactor
        context.coordinator.clearSyncHighlight()
        context.coordinator.renderedData = data
        context.coordinator.renderedTarget = target
        view.document = PDFDocument(data: data)
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
        NotificationCenter.default.removeObserver(coordinator)
        if let monitor = coordinator.syncMonitor {
            NSEvent.removeMonitor(monitor)
        }
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
    }
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
