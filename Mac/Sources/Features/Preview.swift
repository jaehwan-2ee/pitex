import PDFKit
import SwiftUI
import SyncTeXCore

/// Right-hand inspector column: the PDF preview only. The assistant moved
/// into the bottom console so both can stay visible at once.
struct Preview: View {
    @ObservedObject var workspace: WorkspaceModel
    @ObservedObject private var settings: SettingsStore

    init(workspace: WorkspaceModel) {
        self.workspace = workspace
        _settings = ObservedObject(wrappedValue: workspace.settings)
    }

    var body: some View {
        preview
            .frame(minWidth: 260, maxHeight: .infinity, alignment: .top)
            .accessibilityIdentifier("pitex.inspector")
    }

    // MARK: - Preview

    @ViewBuilder
    private var preview: some View {
        VStack(spacing: 0) {
            syncTeXStatus
            Divider()
            switch workspace.buildState {
            case let .succeeded(pdfData, _):
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
                        downloadPDF(pdfData)
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
                PDFDocumentView(
                    data: pdfData,
                    highlightSync: settings.forwardSyncHighlight,
                    onInverseSync: { page, point in
                        Task { await workspace.syncInverse(page: page, point: point) }
                    }
                )
                .frame(maxHeight: .infinity)
                .accessibilityIdentifier("pitex.pdf")
            case .building, .failed, .unavailable:
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

    /// Name of the rendered PDF, matching the built target (or active doc).
    private var pdfDisplayName: String {
        workspace.latestBuiltPDFName ?? "document.pdf"
    }

    /// On-disk URL of the last built PDF — the SyncTeX binding carries it
    /// once a build has succeeded.
    private var pdfFileURL: URL? {
        workspace.syncTeXBinding?.pdfURL
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
        context.coordinator.onInverseSync = onInverseSync
        context.coordinator.highlightSync = highlightSync

        NotificationCenter.default.addObserver(
            context.coordinator,
            selector: #selector(Coordinator.highlightRequested(_:)),
            name: .syncTeXHighlightRequested,
            object: nil
        )
        return view
    }

    func updateNSView(_ view: PDFView, context: Context) {
        context.coordinator.onInverseSync = onInverseSync
        context.coordinator.highlightSync = highlightSync
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
        // early; only genuinely new build output reloads the view.
        if unchanged || context.coordinator.renderedData == data { return }
        context.coordinator.clearSyncHighlight()
        context.coordinator.renderedData = data
        view.document = PDFDocument(data: data)
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
