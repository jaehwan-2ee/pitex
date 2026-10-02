import CoreGraphics
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
                embeddedStatusLine
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
            Text(verbatim: status.text)
                .font(.caption)
                .foregroundStyle(status.color)
                .lineLimit(2)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 10)
                .padding(.vertical, 4)
                .accessibilityIdentifier("pitex.preview.embeddedStatus")
            Divider()
        }
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
        case let .failed(message): return (joined("preview.embedded_failed", message), .red)
        case let .unavailable(reason): return (joined("preview.embedded_unavailable", reason), .orange)
        case .unsupportedRemote: return (String(localized: "preview.embedded_unsupported_remote"), .secondary)
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

/// Offline pdf.js/WKWebView renderer (replaces PDFKit's PDFView). The
/// standalone feasibility run (r5) proved the behaviors this view needs:
/// a real module worker under a custom scheme, CSP-clean loading,
/// geometry/rotate/crop, Cmd-click inverse, UserUnit, and position keep
/// across same-target swaps. Detach/attach mounts a fresh view (SwiftUI
/// dismantle → makeNSView), which the F1 ready-queue makes safe.
///
/// Everything lives behind the same boundary as before: `data`/`target`
/// swaps in updateNSView, SyncTeX highlight notifications, and the
/// Cmd+click → onInverseSync contract. No PDFKit.
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

    /// This WKWebView's JSC lacks ReadableStream's async iterator —
    /// pdf.js getTextContent/find use `for await` on it (proven by the
    /// standalone run: `__pitexPolyfill.needed == true`). Injected into
    /// the page world before any pdf.js code runs. The worker-side site
    /// degrades gracefully on its own (sync fallback).
    private static let streamIteratorPolyfill = """
        (() => {
          const P = typeof ReadableStream === "function" ? ReadableStream.prototype : null;
          const needed = !!P && typeof P[Symbol.asyncIterator] !== "function";
          if (needed) {
            // ponytail: minimal Streams async iterator; no cancel() on early exit (pdf.js consumes to end or error)
            P[Symbol.asyncIterator] = async function* () {
              const reader = this.getReader();
              try { for (;;) { const { done, value } = await reader.read(); if (done) return; yield value; } }
              finally { reader.releaseLock(); }
            };
          }
          window.__pitexPolyfill = { readableStream: !!P, needed };
        })();
        """

    func makeNSView(context: Context) -> WKWebView {
        let coordinator = context.coordinator
        coordinator.renderedData = data
        coordinator.renderedTarget = target
        coordinator.onInverseSync = onInverseSync
        coordinator.highlightSync = highlightSync

        let content = WKUserContentController()
        // The content controller retains its handlers — proxy weakly.
        content.add(WeakScriptMessageHandler(coordinator), name: "pitexPdf")
        content.addUserScript(WKUserScript(
            source: Self.streamIteratorPolyfill,
            injectionTime: .atDocumentStart,
            forMainFrameOnly: true,
            in: .page))

        let configuration = WKWebViewConfiguration()
        configuration.userContentController = content
        configuration.setURLSchemeHandler(coordinator.scheme, forURLScheme: "pitex-pdfjs")

        let view = PDFJSWebView(frame: .zero, configuration: configuration)
        view.navigationDelegate = coordinator
        coordinator.webView = view
        view.addGestureRecognizer(NSMagnificationGestureRecognizer(
            target: coordinator, action: #selector(Coordinator.handleMagnify(_:))))
        // Zoom step parity with the PDFView control (2^(1/4) ≈ 1.1892 measured
        // in C-v7; FLAGGED — confirmed by the C-v9 control in≥2/out≥2 series).
        let zoomStep = pow(2.0, 0.25)
        view.onZoomIn = { [weak coordinator] in coordinator?.zoomBy(zoomStep) }
        view.onZoomOut = { [weak coordinator] in coordinator?.zoomBy(1.0 / zoomStep) }
        view.onAutoResize = { [weak coordinator] in coordinator?.autoResize() }
        view.load(URLRequest(url: URL(string: "pitex-pdfjs://app/viewer.html")!))

        coordinator.load(data: data, sameTarget: false)

        NotificationCenter.default.addObserver(
            coordinator,
            selector: #selector(Coordinator.highlightRequested(_:)),
            name: .syncTeXHighlightRequested,
            object: workspace
        )
        return view
    }

    func updateNSView(_ view: WKWebView, context: Context) {
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
        if sameTarget && (unchanged || context.coordinator.renderedData == data) { return }
        context.coordinator.renderedData = data
        context.coordinator.renderedTarget = target
        context.coordinator.load(data: data, sameTarget: sameTarget)
        NSLog("[preview] pdf.js document swap target=%@ bytes=%d gen=%d",
              target, data.count, context.coordinator.generation)
    }

    static func dismantleNSView(_ view: WKWebView, coordinator: Coordinator) {
        NSLog("[preview] teardown target=%@", coordinator.renderedTarget)
        NotificationCenter.default.removeObserver(coordinator)
        // Best-effort page shutdown (worker destroy). The web view's
        // teardown kills it anyway; this just closes the loop cleanly.
        view.callAsyncJavaScript(
            "return window.pitex?.shutdown?.() ?? true;",
            in: nil, in: .page, completionHandler: { _ in })
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    /// Serves the bundled pdf.js payload plus per-generation document
    /// bytes on `pitex-pdfjs://`. Retention is lifecycle-driven: the
    /// registry holds only the displayed, in-flight and pending
    /// generations; older ones are evicted when a newer gen displays
    /// (superseded loads destroy their own tasks JS-side). `Data` shares
    /// storage with `retainedPDF`, so holding the current gen is free —
    /// the memory risk is the WebContent process, which the perf gate
    /// measures separately.
    @MainActor
    final class PDFJSSchemeHandler: NSObject, WKURLSchemeHandler {
        private var generations: [Int: Data] = [:]

        func publish(_ gen: Int, data: Data) {
            generations[gen] = data
        }

        /// F5: on `displayed(g)` only strictly-older gens are dead;
        /// displayed/in-flight/pending keep their bytes. `Int.max`
        /// clears everything (post-termination).
        func evictBelow(_ g: Int) {
            for key in generations.keys where key < g {
                generations.removeValue(forKey: key)
            }
        }

        func webView(_ webView: WKWebView, start task: WKURLSchemeTask) {
            let path = task.request.url?.path ?? ""
            let data: Data
            let type: String
            if path.hasPrefix("/doc/") {
                let gen = Int(path.dropFirst(5).dropLast(4)) ?? -1
                guard let d = generations[gen] else {
                    task.didFailWithError(URLError(.fileDoesNotExist)); return
                }
                data = d; type = "application/pdf"
            } else {
                let rel = String(path.dropFirst())
                guard let base = Bundle.main.resourceURL?
                        .appendingPathComponent("pdfjs")
                        .standardizedFileURL else {
                    task.didFailWithError(URLError(.fileDoesNotExist)); return
                }
                let full = base.appendingPathComponent(rel).standardizedFileURL
                // serve only files inside the pdfjs resource dir (no
                // traversal out of the bundle)
                guard full.path.hasPrefix(base.path + "/"),
                      let d = try? Data(contentsOf: full) else {
                    task.didFailWithError(URLError(.fileDoesNotExist)); return
                }
                data = d
                type = rel.hasSuffix(".mjs") ? "text/javascript"
                     : rel.hasSuffix(".js")  ? "text/javascript"
                     : rel.hasSuffix(".css") ? "text/css"
                     : rel.hasSuffix(".wasm") ? "application/wasm"
                     : rel.hasSuffix(".pdf") ? "application/pdf"
                     : rel.hasSuffix(".html") ? "text/html"
                     : "application/octet-stream"
            }
            // Whole response delivered synchronously inside start() — a
            // stop() can never interleave between didReceive/didFinish.
            task.didReceive(URLResponse(url: task.request.url!, mimeType: type,
                                        expectedContentLength: data.count,
                                        textEncodingName: nil))
            task.didReceive(data)
            task.didFinish()
        }

        func webView(_ webView: WKWebView, stop task: WKURLSchemeTask) {}
    }

    /// pdf.js preview surface. The context menu gains Zoom In / Zoom Out /
    /// Automatically Resize — the PDFView parity items (F7; the control's
    /// stock menu is confirmed in the parity screenshot gate). No Reload
    /// item is filtered: recovery is self-healing — every `ready`
    /// re-pushes the current document, whatever caused it (F6).
    final class PDFJSWebView: WKWebView {
        var onZoomIn: () -> Void = {}
        var onZoomOut: () -> Void = {}
        var onAutoResize: () -> Void = {}

        override func willOpenMenu(_ menu: NSMenu, with event: NSEvent) {
            menu.addItem(.separator())
            let zoomIn = NSMenuItem(title: String(localized: "preview.zoom_in"),
                action: #selector(zoomInAction), keyEquivalent: "")
            zoomIn.target = self
            let zoomOut = NSMenuItem(title: String(localized: "preview.zoom_out"),
                action: #selector(zoomOutAction), keyEquivalent: "")
            zoomOut.target = self
            let auto = NSMenuItem(title: String(localized: "preview.zoom_auto"),
                action: #selector(autoResizeAction), keyEquivalent: "")
            auto.target = self
            menu.addItem(zoomIn); menu.addItem(zoomOut); menu.addItem(auto)
            super.willOpenMenu(menu, with: event)
        }
        @objc private func zoomInAction() { onZoomIn() }
        @objc private func zoomOutAction() { onZoomOut() }
        @objc private func autoResizeAction() { onAutoResize() }
    }

    @MainActor
    final class Coordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
        weak var webView: WKWebView?
        let scheme = PDFJSSchemeHandler()
        var renderedData = Data()
        /// Workspace + source identity of the rendered document —
        /// distinguishes a rebuild of the same target (preserve
        /// position) from a target or workspace switch (reset).
        var renderedTarget = ""
        var generation = 0
        /// True once the page posted `ready`; loads queue until then.
        private(set) var ready = false
        private var pendingLoad: (data: Data, sameTarget: Bool)?
        var onInverseSync: (Int, SyncTeXCore.PDFPoint) -> Void = { _, _ in }
        var highlightSync = false {
            didSet { if oldValue != highlightSync { pushHighlightSync() } }
        }

        /// MediaBox list via the CG parser — the same geometry oracle the
        /// proof used; pdf.js needs user-space boxes for inverse + goto.
        private func mediaBoxes(_ data: Data) -> [[Double]] {
            guard let dp = CGDataProvider(data: data as CFData),
                  let doc = CGPDFDocument(dp), doc.numberOfPages > 0 else { return [] }
            return (1...doc.numberOfPages).map { i in
                let r = doc.page(at: i)?.getBoxRect(.mediaBox) ?? .zero
                return [Double(r.minX), Double(r.minY), Double(r.maxX), Double(r.maxY)]
            }
        }

        private func js(_ body: String, arguments: [String: Any] = [:]) {
            webView?.callAsyncJavaScript(
                body, arguments: arguments, in: nil, in: .page,
                completionHandler: { r in
                    if case let .failure(e) = r {
                        NSLog("[preview] js error: %@", "\(e)")
                    }
                })
        }

        /// Queue-or-fire document push. `sameTarget` tells the JS side to
        /// capture the current position before the swap and restore it on
        /// pagesinit (proven by the standalone position-keep check).
        func load(data: Data, sameTarget: Bool) {
            // Before `ready`, window.pitex doesn't exist — queue newest
            // wins (F1). Generation only counts pushes that reached JS.
            guard ready else { pendingLoad = (data, sameTarget); return }
            generation += 1
            scheme.publish(generation, data: data)
            js("return window.pitex.load(gen, mediaBoxes, sameTarget);",
               arguments: ["gen": generation,
                           "mediaBoxes": mediaBoxes(data),
                           "sameTarget": sameTarget])
        }

        func pushHighlightSync() {
            guard ready else { return }
            js("return window.pitex.setHighlightSync(on);",
               arguments: ["on": highlightSync])
        }

        /// Pinch-magnify → pdf.js scale (F4): per-event factor, passed as
        /// an argument; pdf.js re-renders at the new resolution.
        @objc func handleMagnify(_ g: NSMagnificationGestureRecognizer) {
            let factor = 1 + g.magnification
            g.magnification = 0
            js("return window.pitex.zoomBy(f);", arguments: ["f": factor])
        }

        /// WebContent died → reload the page. `ready` posts again and the
        /// pending-document repush heals the view (F6 public path).
        func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
            NSLog("[preview] pdf.js WebContent terminated; reloading")
            ready = false
            scheme.evictBelow(Int.max)
            pendingLoad = nil    // `ready` re-pushes renderedData
            webView.reload()
        }

        /// Context-menu zoom parity (F7): PDFView's Zoom In/Out steps.
        func zoomBy(_ factor: Double) {
            js("return window.pitex.zoomBy(f);", arguments: ["f": factor])
        }
        /// "Automatically Resize" = the PDFView default scale mode ≈
        /// pdf.js page-width fit.
        func autoResize() {
            js("return window.pitex.autoResize();")
        }


        // MARK: WKScriptMessageHandler — the page reports inverse sync,
        // links, errors.

        func userContentController(
            _ controller: WKUserContentController,
            didReceive message: WKScriptMessage
        ) {
            guard let m = message.body as? [String: Any],
                  let type = m["type"] as? String else { return }
            switch type {
            case "ready":
                ready = true
                // Observe the polyfill outcome — missing record means the
                // user script never ran (loud, not silent).
                webView?.callAsyncJavaScript(
                    "return window.__pitexPolyfill ?? null;",
                    arguments: [:], in: nil, in: .page) { r in
                        if case let .success(v) = r {
                            NSLog("[preview] pdf.js stream-polyfill %@", "\(v)")
                        }
                    }
                pushHighlightSync()
                // F6 public recovery: every `ready` — initial, menu
                // reload, or post-crash reload — re-pushes the queued or
                // current document.
                if let p = pendingLoad ?? (renderedData.isEmpty ? nil : (renderedData, false)) {
                    pendingLoad = nil
                    load(data: p.data, sameTarget: p.sameTarget)
                }
            case "displayed":
                // F5: evict only strictly-older gens — an in-flight middle
                // gen must keep its bytes or its fetch 404s.
                if let g = m["gen"] as? Int {
                    scheme.evictBelow(g)
                }
            case "load-error", "destroy-error":
                NSLog("[preview] pdf.js %@: %@", type, "\(m)")
            case "inverse":
                guard let page = m["page"] as? Int,
                      let x = m["x"] as? Double, let y = m["y"] as? Double,
                      let point = try? SyncTeXCore.PDFPoint(x: x, y: y)
                else { return }
                onInverseSync(page, point)
            case "link":
                if let u = m["url"] as? String, let url = URL(string: u),
                   url.scheme == "https" || url.scheme == "http" {
                    NSWorkspace.shared.open(url)
                }
            case "jserror", "csp":
                NSLog("[preview] pdf.js %@: %@", type, "\(m)")
            default: break
            }
        }

        // MARK: WKNavigationDelegate — only our own scheme navigates.

        func webView(
            _ webView: WKWebView,
            decidePolicyFor action: WKNavigationAction
        ) async -> WKNavigationActionPolicy {
            action.request.url?.scheme == "pitex-pdfjs" ? .allow : .cancel
        }

        /// Forward SyncTeX: scroll ALWAYS, overlay only under
        /// highlightSync (the JS side gates the highlight itself).
        @objc func highlightRequested(_ notification: Notification) {
            guard let info = notification.userInfo,
                  let page = info["page"] as? Int else { return }
            let x = info["x"] as? Double ?? 0
            let v = info["y"] as? Double ?? 0
            let w = info["width"] as? Double ?? 0
            let h = info["height"] as? Double ?? 0
            js("return window.pitex.goto(gen, p, x, v, w, h);",
               arguments: ["gen": generation, "p": page,
                           "x": x, "v": v, "w": w, "h": h])
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
