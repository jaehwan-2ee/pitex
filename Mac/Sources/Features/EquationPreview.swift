import AppKit
import BuildCore
import Combine
import EditorFeature
import LanguageCore
import PDFKit
import WebKit

// Overleaf-style equation preview for the macOS editor: the portable
// `EquationPreviewEngine` decides what to show; this file hosts it —
// one persistent MathJax WKWebView (served from the bundled asset tree over
// `pitex-equation://preview/`), a non-activating NSPopover anchored to the
// source range, the optional exact TeX renderer and the editor hooks.

// MARK: - Assets

/// Serves `Resources/equation-preview/**` (html/js/css only) and nothing
/// else. Paths are canonicalized and must stay inside the tree.
@MainActor
final class EquationAssetSchemeHandler: NSObject, WKURLSchemeHandler {
    static let scheme = "pitex-equation"
    static let pageURL = URL(string: "pitex-equation://preview/renderer.html")!
    private static let mimeTypes = ["html": "text/html", "js": "text/javascript", "css": "text/css"]

    private let root: URL?

    override init() {
        root = Bundle.main.resourceURL?
            .appendingPathComponent("equation-preview", isDirectory: true)
            .resolvingSymlinksInPath()
            .standardizedFileURL
        super.init()
    }

    /// The allowlisted file for `url`, or nil.
    func file(for url: URL) -> URL? {
        guard let root, url.scheme == Self.scheme, url.host == "preview",
              url.query == nil, url.user == nil, url.port == nil else { return nil }
        let path = url.path
        guard path.hasPrefix("/"), !path.contains("\\"), !path.contains("\u{0}") else { return nil }
        let components = path.split(separator: "/", omittingEmptySubsequences: false).dropFirst()
        guard !components.isEmpty, !components.contains(where: { $0.isEmpty || $0 == "." || $0 == ".." }) else { return nil }
        let file = components.reduce(root) { $0.appendingPathComponent(String($1)) }
            .resolvingSymlinksInPath().standardizedFileURL
        guard file.path.hasPrefix(root.path + "/"),
              Self.mimeTypes[file.pathExtension.lowercased()] != nil else { return nil }
        return file
    }

    func webView(_ webView: WKWebView, start urlSchemeTask: WKURLSchemeTask) {
        guard let url = urlSchemeTask.request.url, let file = file(for: url),
              let data = FileManager.default.contents(atPath: file.path) else {
            urlSchemeTask.didFailWithError(URLError(.fileDoesNotExist))
            return
        }
        let response = URLResponse(
            url: url, mimeType: Self.mimeTypes[file.pathExtension.lowercased()],
            expectedContentLength: data.count, textEncodingName: "utf-8"
        )
        urlSchemeTask.didReceive(response)
        urlSchemeTask.didReceive(data)
        urlSchemeTask.didFinish()
    }

    func webView(_ webView: WKWebView, stop urlSchemeTask: WKURLSchemeTask) {}
}

// MARK: - Renderer

/// Rendering and wheel scrolling remain available, but an equation
/// preview must never enter the editor's first-responder chain.
@MainActor
private final class EquationPreviewWebView: WKWebView {
    override var acceptsFirstResponder: Bool { false }
    override func becomeFirstResponder() -> Bool { false }
}

/// One persistent hidden WKWebView with MathJax loaded once; the same view
/// is the popover's display surface. Calls pass one JSON argument that the
/// page parses — TeX never becomes script text.
@MainActor
final class MathJaxEquationRenderer: NSObject, WKNavigationDelegate, WKUIDelegate {
    static let renderTimeout: Duration = .seconds(5)
    /// Web processes may restart a few times in a row; a page that never
    /// reaches ready stops replacing itself so a persistent failure can't
    /// respawn forever.
    static let maxRestarts = 3

    private(set) var webView: WKWebView!
    private let schemeHandler = EquationAssetSchemeHandler()
    private var loaded = false
    /// Page identity once `pitexEquation.ready` resolved.
    private(set) var version: String?
    /// Restarts since the last ready page; `recover()` (explicit off→on) is
    /// the only re-arm. A render that misses its deadline spends this same
    /// budget: a slow-but-live page can exhaust it, which is the deliberate
    /// tradeoff — the finite ceiling beats an unbounded respawn loop (N4).
    private var failures = 0
    var onReady: ((String) -> Void)?
    var onUnavailable: (() -> Void)?
    /// A replacement web view was created (hung or crashed page).
    var onWebViewReplaced: (() -> Void)?

    override init() {
        super.init()
        webView = makeWebView()
        webView.load(URLRequest(url: EquationAssetSchemeHandler.pageURL))
    }

    private func makeWebView() -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.setURLSchemeHandler(schemeHandler, forURLScheme: EquationAssetSchemeHandler.scheme)
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = false
        configuration.mediaTypesRequiringUserActionForPlayback = .all
        let view = EquationPreviewWebView(frame: NSRect(x: 0, y: 0, width: 320, height: 80), configuration: configuration)
        view.navigationDelegate = self
        view.uiDelegate = self
        view.allowsMagnification = false
        view.allowsBackForwardNavigationGestures = false
        view.allowsLinkPreview = false
        // The popover container is the accessibility element.
        view.setAccessibilityElement(false)
        return view
    }

    /// Discards a hung/crashed page: releasing the view ends its process.
    /// Beyond `maxRestarts` consecutive failures the renderer stays down —
    /// `recover()` (explicit off→on) is the only re-arm (B5).
    private func replaceWebView() {
        failures += 1
        loaded = false
        version = nil
        webView.navigationDelegate = nil
        webView.uiDelegate = nil
        webView.removeFromSuperview()
        onUnavailable?()
        guard failures < Self.maxRestarts else { return }
        webView = makeWebView()
        onWebViewReplaced?()
        webView.load(URLRequest(url: EquationAssetSchemeHandler.pageURL))
    }

    /// Explicit user re-enable: re-arms the crash budget and rebuilds the
    /// web view when the cap left it exhausted. Ordinary attach/settings
    /// pushes must not replenish the budget (B5).
    func recover() {
        failures = 0
        // The cap leaves `webView` pointing at a view whose page we tore
        // down; rebuild it so the re-enabled preview actually comes back.
        if version == nil {
            webView.navigationDelegate = nil
            webView.uiDelegate = nil
            webView.removeFromSuperview()
            loaded = false
            webView = makeWebView()
            onWebViewReplaced?()
            webView.load(URLRequest(url: EquationAssetSchemeHandler.pageURL))
        }
    }

    /// `body` receives the parsed `payload`; its JSON-stringified result
    /// comes back. Nil on timeout, bridge error or a replaced page.
    private func call(_ body: String, payload: [String: Any], timeout: Duration) async -> [String: Any]? {
        guard let data = try? JSONSerialization.data(withJSONObject: payload),
              let json = String(data: data, encoding: .utf8) else { return nil }
        let view: WKWebView = webView
        let reply: String?? = await Self.withTimeout(timeout) {
            try? await view.callAsyncJavaScript(
                "const input = JSON.parse(payload); return JSON.stringify(await (\(body)));",
                arguments: ["payload": json], in: nil, contentWorld: .page
            ) as? String
        }
        guard let reply else {
            // The page missed its deadline, so we replace the view. Caveat:
            // the awaiting Task still strongly retains the old WKWebView (and
            // WKWebView has no public kill API), so a truly infinite JS loop
            // pins the dead process until the call returns or the app quits.
            // `maxBuffer`/input bounds mean only pathological content reaches
            // that; finite-but-slow calls eventually release (C3).
            if view === webView { replaceWebView() }
            return nil
        }
        guard let text = reply, let object = try? JSONSerialization.jsonObject(with: Data(text.utf8)) else { return nil }
        return object as? [String: Any]
    }

    func render(_ request: EquationRenderRequest) async -> EquationRenderOutcome {
        guard version != nil else { return .failed(.rendererFailed) }
        let payload: [String: Any] = [
            "source": request.source,
            "displayMode": request.displayMode,
            "contextKey": request.contextKey,
            "definitions": request.definitions,
            "fontSize": request.fontSize,
        ]
        guard let reply = await call("window.pitexEquation.render(input)", payload: payload, timeout: Self.renderTimeout) else {
            return .failed(.rendererFailed)
        }
        if reply["ok"] as? Bool == true, let svg = reply["svg"] as? String { return .svg(svg) }
        switch reply["error"] as? String {
        case "undefined": return .failed(.undefinedCommand)
        case "limit": return .failed(.tooLarge)
        default: return .failed(.invalid)
        }
    }

    /// Puts sanitized SVG into the display element; its CSS size in points.
    func show(svg: String, foreground: String, background: String, fontSize: Double) async -> CGSize? {
        guard version != nil else { return nil }
        let payload: [String: Any] = [
            "svg": svg, "fontSize": fontSize,
            "theme": ["foreground": foreground, "background": background],
        ]
        guard let reply = await call("window.pitexEquation.show(input)", payload: payload, timeout: .seconds(2)),
              reply["ok"] as? Bool == true,
              let width = (reply["width"] as? NSNumber)?.doubleValue,
              let height = (reply["height"] as? NSNumber)?.doubleValue else { return nil }
        return CGSize(width: width, height: height)
    }

    /// First of `operation` and the timeout (a hung page cannot hold the
    /// caller; its late reply is dropped).
    private static func withTimeout<T: Sendable>(_ duration: Duration, _ operation: @escaping @MainActor () async -> T) async -> T? {
        await withCheckedContinuation { (continuation: CheckedContinuation<T?, Never>) in
            let once = ResumeOnce(continuation)
            Task { @MainActor in once.resume(await operation()) }
            Task {
                try? await Task.sleep(for: duration)
                once.resume(nil)
            }
        }
    }

    // MARK: WKNavigationDelegate / WKUIDelegate — only the page load navigates.

    func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction) async -> WKNavigationActionPolicy {
        guard !loaded, navigationAction.targetFrame?.isMainFrame == true,
              navigationAction.request.url == EquationAssetSchemeHandler.pageURL else { return .cancel }
        return .allow
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard webView === self.webView else { return }
        loaded = true
        Task { @MainActor [weak self] in
            guard let self else { return }
            let reply = await call("window.pitexEquation.ready.then((identity) => ({ identity }))",
                                   payload: [:], timeout: .seconds(10))
            guard webView === self.webView else { return }
            if let identity = reply?["identity"] as? String {
                version = identity
                // No reset here: a page that reaches ready and then dies
                // mid-render must still count toward the restart cap.
                onReady?(identity)
            } else {
                onUnavailable?()
            }
        }
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        if webView === self.webView { onUnavailable?() }
    }

    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
        if webView === self.webView { onUnavailable?() }
    }

    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        if webView === self.webView { replaceWebView() }
    }

    func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration,
                 for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? { nil }
}

/// Resumes a continuation exactly once from whichever side finishes first.
private final class ResumeOnce<T: Sendable>: @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: CheckedContinuation<T?, Never>?
    init(_ continuation: CheckedContinuation<T?, Never>) { self.continuation = continuation }
    func resume(_ value: T?) {
        lock.lock()
        let pending = continuation
        continuation = nil
        lock.unlock()
        pending?.resume(returning: value)
    }
}

// MARK: - Exact preview image

/// A finished bitmap crossing from the crop task — CGImage is immutable.
struct CroppedImage: @unchecked Sendable {
    let image: CGImage
}

enum ExactEquationImage {
    /// Renders page 1 on white paper at `scale` and crops to the ink.
    static func cropped(pdf: [UInt8], scale: CGFloat) -> CGImage? {
        guard let document = PDFDocument(data: Data(pdf)), let page = document.page(at: 0) else { return nil }
        let bounds = page.bounds(for: .cropBox)
        let width = Int(bounds.width * scale), height = Int(bounds.height * scale)
        guard width > 0, height > 0, width * height <= 40_000_000,
              let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
                                      space: CGColorSpaceCreateDeviceRGB(),
                                      bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        context.setFillColor(CGColor(gray: 1, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: width, height: height))
        context.scaleBy(x: scale, y: scale)
        page.draw(with: .cropBox, to: context)
        guard let image = context.makeImage(), let data = context.data else { return nil }
        let pixels = data.bindMemory(to: UInt8.self, capacity: width * height * 4)
        var minX = width, minY = height, maxX = -1, maxY = -1
        for y in 0..<height {
            let row = pixels + y * width * 4
            for x in 0..<width where row[x * 4] < 245 || row[x * 4 + 1] < 245 || row[x * 4 + 2] < 245 {
                minX = min(minX, x); maxX = max(maxX, x)
                minY = min(minY, y); maxY = max(maxY, y)
            }
        }
        guard maxX >= minX, maxY >= minY else { return nil }
        let pad = Int(6 * scale)
        let left = max(minX - pad, 0), top = max(minY - pad, 0)
        // Bitmap memory rows run top-down, as makeImage's pixel rows do.
        return image.cropping(to: CGRect(x: left, y: top,
                                         width: min(maxX + pad, width - 1) - left + 1,
                                         height: min(maxY + pad, height - 1) - top + 1))
    }
}

// MARK: - Popover

/// Swallows clicks (the preview never takes focus or navigates) while
/// forwarding wheel scrolling to the visible content.
private final class EquationPreviewContentView: NSView {
    var onPointerInside: ((Bool) -> Void)?
    weak var scrollTarget: NSView?
    private var tracking: NSTrackingArea?

    override var acceptsFirstResponder: Bool { false }
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { false }
    override var mouseDownCanMoveWindow: Bool { false }
    override func hitTest(_ point: NSPoint) -> NSView? {
        let local = superview.map { convert(point, from: $0) } ?? point
        return bounds.contains(local) ? self : nil
    }
    override func mouseDown(with event: NSEvent) {}
    override func rightMouseDown(with event: NSEvent) {}
    override func otherMouseDown(with event: NSEvent) {}
    override func scrollWheel(with event: NSEvent) { scrollTarget?.scrollWheel(with: event) }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let tracking { removeTrackingArea(tracking) }
        let area = NSTrackingArea(rect: .zero, options: [.mouseEnteredAndExited, .activeAlways, .inVisibleRect], owner: self)
        addTrackingArea(area)
        tracking = area
    }
    override func mouseEntered(with event: NSEvent) { onPointerInside?(true) }
    override func mouseExited(with event: NSEvent) { onPointerInside?(false) }
}

@MainActor
final class EquationPreviewPopover: NSObject, NSPopoverDelegate {
    static let maximumWidth: CGFloat = 720
    /// Height cap: a fraction of the host screen so many short rows fit
    /// without an internal scroller, still hard-bounded for small screens.
    static let maximumHeightFraction: CGFloat = 0.55
    static let maximumHeightCeiling: CGFloat = 720
    static func maximumHeight(for screen: NSScreen?) -> CGFloat {
        let visible = (screen ?? NSScreen.main)?.visibleFrame.height ?? maximumHeightCeiling
        return min(max(360, visible * maximumHeightFraction), maximumHeightCeiling)
    }
    static let padding: CGFloat = 12
    static let badgeHeight: CGFloat = 16

    enum Body {
        case fast(size: CGSize)
        case exact(CGImage, scale: CGFloat)
        case message(String)
    }

    private let popover = NSPopover()
    private let content = EquationPreviewContentView()
    private let badge = NSTextField(labelWithString: "")
    private let message = NSTextField(labelWithString: "")
    private let imageScroll = NSScrollView()
    private let imageView = NSImageView()
    private weak var webView: WKWebView?
    private var closingOnPurpose = false
    var onPointerInside: ((Bool) -> Void)? {
        didSet { content.onPointerInside = onPointerInside }
    }

    override init() {
        super.init()
        popover.behavior = .applicationDefined
        popover.delegate = self
        let controller = NSViewController()
        controller.view = content
        popover.contentViewController = controller
        content.wantsLayer = true
        content.setAccessibilityElement(true)
        content.setAccessibilityRole(.image)
        content.setAccessibilityLabel(String(localized: "equation_preview.accessibility_label"))
        content.setAccessibilityIdentifier("pitex.equationPreview")

        badge.font = .systemFont(ofSize: NSFont.smallSystemFontSize - 1)
        badge.textColor = .secondaryLabelColor
        badge.alignment = .right
        badge.setAccessibilityElement(false)
        message.font = .systemFont(ofSize: NSFont.systemFontSize)
        message.textColor = .secondaryLabelColor
        message.setAccessibilityElement(false)
        imageScroll.documentView = imageView
        imageScroll.hasVerticalScroller = true
        imageScroll.hasHorizontalScroller = true
        imageScroll.autohidesScrollers = true
        imageScroll.drawsBackground = true
        imageScroll.backgroundColor = .white
        imageScroll.borderType = .noBorder
        imageView.imageScaling = .scaleNone
        for view in [imageScroll, message, badge] as [NSView] { content.addSubview(view) }
    }

    var isShown: Bool { popover.isShown }

    func attach(webView: WKWebView) {
        guard self.webView !== webView else { return }
        self.webView?.removeFromSuperview()
        self.webView = webView
        webView.isHidden = true
        content.addSubview(webView, positioned: .below, relativeTo: nil)
    }

    /// Shows `body` next to `rect` (text-view coordinates, flipped).
    func present(_ body: Body, label: String, source: String, rect: NSRect, in view: NSView,
                 placement: EquationPreviewPlacement, background: NSColor) {
        var size: CGSize
        webView?.isHidden = true
        imageScroll.isHidden = true
        message.isHidden = true
        switch body {
        case let .fast(pageSize):
            size = pageSize
            webView?.isHidden = false
            content.scrollTarget = webView
            content.setAccessibilityRole(.image)
        case let .exact(image, scale):
            let points = CGSize(width: CGFloat(image.width) / scale, height: CGFloat(image.height) / scale)
            imageView.image = NSImage(cgImage: image, size: points)
            imageView.frame = NSRect(origin: .zero, size: points)
            size = CGSize(width: points.width + Self.padding, height: points.height + Self.padding)
            imageScroll.isHidden = false
            content.scrollTarget = imageScroll
            content.setAccessibilityRole(.image)
        case let .message(text):
            message.stringValue = text
            message.sizeToFit()
            size = CGSize(width: message.frame.width + 2 * Self.padding, height: message.frame.height + 2 * Self.padding)
            message.isHidden = false
            content.scrollTarget = nil
            content.setAccessibilityRole(.staticText)
        }
        badge.stringValue = label
        badge.isHidden = label.isEmpty
        let badgeHeight = label.isEmpty ? 0 : Self.badgeHeight
        let maxBodyHeight = Self.maximumHeight(for: view.window?.screen) - badgeHeight
        // A classic (legacy) scroller takes its thickness out of the viewport
        // it sits in: a vertical one squeezes the width (summoning a
        // horizontal one) and a horizontal one squeezes the height (summoning
        // a vertical one). Reserve it whenever the content will scroll on
        // that axis. Overlay scrollers float over the content and reserve
        // nothing; scrollerWidth still reports 17 for them (measured), hence
        // the style check.
        let thickness: CGFloat = NSScroller.preferredScrollerStyle == .legacy
            ? NSScroller.scrollerWidth(for: .regular, scrollerStyle: .legacy) : 0
        var scrollsVertically = size.height > maxBodyHeight
        var scrollsHorizontally = size.width > Self.maximumWidth
        scrollsVertically = scrollsVertically || size.height + (scrollsHorizontally ? thickness : 0) > maxBodyHeight
        scrollsHorizontally = scrollsHorizontally || size.width + (scrollsVertically ? thickness : 0) > Self.maximumWidth
        // Clamp to 720pt wide and the screen-relative height; larger
        // expressions scroll inside.
        let bodySize = CGSize(width: min(max(size.width + (scrollsVertically ? thickness : 0), 80), Self.maximumWidth),
                              height: min(size.height + (scrollsHorizontally ? thickness : 0), maxBodyHeight))
        let total = CGSize(width: bodySize.width, height: bodySize.height + badgeHeight)
        let bodyFrame = NSRect(x: 0, y: badgeHeight, width: bodySize.width, height: bodySize.height)
        webView?.frame = bodyFrame
        imageScroll.frame = bodyFrame.insetBy(dx: Self.padding / 2, dy: Self.padding / 2)
        message.frame = NSRect(x: Self.padding, y: bodyFrame.minY + Self.padding,
                               width: bodyFrame.width - 2 * Self.padding, height: message.frame.height)
        badge.frame = NSRect(x: Self.padding, y: 1, width: total.width - 2 * Self.padding, height: max(badgeHeight - 2, 0))
        content.layer?.backgroundColor = background.cgColor
        content.setAccessibilityValue(source)
        content.setAccessibilityHelp(label)
        popover.animates = !NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        popover.contentSize = total
        if popover.isShown {
            popover.positioningRect = rect
        } else if view.window != nil {
            // Flipped text view: minY is the top edge.
            popover.show(relativeTo: rect, of: view, preferredEdge: placement == .above ? .minY : .maxY)
        }
    }

    func reposition(_ rect: NSRect) {
        guard popover.isShown else { return }
        popover.positioningRect = rect
    }

    func hide() {
        guard popover.isShown else { return }
        closingOnPurpose = true
        // close() animates and isShown stays true until the animation ends
        // (~0.5 s): a present() in that window would only reposition the
        // closing popover and it would vanish. Close without the animation.
        let animates = popover.animates
        popover.animates = false
        popover.close()
        popover.animates = animates
        closingOnPurpose = false
    }

    // The preview is supplemental — nothing but the controller closes it.
    func popoverShouldClose(_ popover: NSPopover) -> Bool { closingOnPurpose }
}

// MARK: - Controller

/// Drives `EquationPreviewEngine` for one workspace: editor events in,
/// renderer/popover/TeX work out. Created with the workspace, rebound to
/// every editor text view, shut down in `close()`.
@MainActor
final class EquationPreviewController {
    struct Workspace {
        /// Unsaved text of an open document, by URL.
        var openText: @MainActor (URL) async -> String? = { _ in nil }
        var projectRoot: URL?
    }

    var workspace = Workspace()
    private var engine = EquationPreviewEngine()
    private lazy var renderer: MathJaxEquationRenderer = makeRenderer()
    private let popover = EquationPreviewPopover()
    private weak var textView: NSTextView?
    private weak var observedWindow: NSWindow?
    private var windowObservation: NSKeyValueObservation?
    /// nonisolated(unsafe): torn down in `detach`/`shutdown`/deinit only.
    nonisolated(unsafe) private var observers: [NSObjectProtocol] = []
    nonisolated(unsafe) private var workspaceObservers: [NSObjectProtocol] = []
    nonisolated(unsafe) private var keyMonitor: Any?
    private var cancellables: [AnyCancellable] = []
    private var hoverTracker: EquationHoverTracker?
    private var wakeTask: Task<Void, Never>?
    private var exactTasks: [String: Task<Void, Never>] = [:]
    private var showToken: UInt64 = 0
    private var presentation: EquationPreviewPresentation?
    private var documentURL: URL?
    /// The current build root file, or nil when the document is its own root.
    private var rootURL: URL?
    private var revision: UInt64 = 0
    private var exactProfile: ExactEquationProfile?
    /// Last pushed enabled flag: the renderer's crash budget recovers only
    /// on the explicit off→on transition (B5).
    private var lastEnabled: Bool?
    /// key → job id: rejects a canceled job's completion that would erase
    /// the replacement under the same key.
    private var exactTaskIDs: [String: UUID] = [:]
    private var externalStamps: [String: Date] = [:]
    private let epoch = ContinuousClock.now

    init() {
        // objectWillChange fires before the new value lands — read it on the
        // next main-queue turn.
        cancellables.append(SettingsStore.shared.objectWillChange
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in MainActor.assumeIsolated { self?.refreshSettings() } })
        cancellables.append(AppearanceSettings.shared.objectWillChange
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in MainActor.assumeIsolated { self?.refreshAppearance() } })
        workspaceObservers.append(NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil, queue: .main
        ) { [weak self] _ in MainActor.assumeIsolated { self?.refreshAppearance() } })
        observersOfApplication.append(NotificationCenter.default.addObserver(
            forName: NSApplication.didBecomeActiveNotification, object: nil, queue: .main
        ) { [weak self] _ in MainActor.assumeIsolated { self?.refreshExternalFiles() } })
        refreshSettings()
        refreshAppearance()
    }

    nonisolated(unsafe) private var observersOfApplication: [NSObjectProtocol] = []

    deinit {
        observers.forEach { NotificationCenter.default.removeObserver($0) }
        observersOfApplication.forEach { NotificationCenter.default.removeObserver($0) }
        workspaceObservers.forEach { NSWorkspace.shared.notificationCenter.removeObserver($0) }
        if let keyMonitor { NSEvent.removeMonitor(keyMonitor) }
    }

    private var nowMs: UInt64 {
        let elapsed = ContinuousClock.now - epoch
        return UInt64(max(elapsed.components.seconds, 0)) * 1_000
            + UInt64(max(elapsed.components.attoseconds, 0) / 1_000_000_000_000_000)
    }

    private func makeRenderer() -> MathJaxEquationRenderer {
        let renderer = MathJaxEquationRenderer()
        renderer.onReady = { [weak self] version in
            guard let self else { return }
            apply(engine.rendererReady(version: version, nowMs: nowMs))
        }
        renderer.onUnavailable = { [weak self] in
            guard let self else { return }
            apply(engine.rendererUnavailable())
        }
        renderer.onWebViewReplaced = { [weak self, weak renderer] in
            guard let self, let renderer else { return }
            popover.attach(webView: renderer.webView)
        }
        popover.attach(webView: renderer.webView)
        popover.onPointerInside = { [weak self] inside in
            guard let self else { return }
            apply(engine.pointerInPopover(inside, nowMs: nowMs))
        }
        return renderer
    }

    // MARK: Workspace inputs

    /// The active document changed (open or tab switch).
    func setDocument(_ url: URL?) {
        documentURL = url?.standardizedFileURL
        revision += 1
        apply(engine.openDocument(fileID: documentURL?.path, revision: revision, nowMs: nowMs))
        refreshSettings()
    }

    /// The build target (pinned or resolved main document).
    func setRootFile(_ url: URL?) {
        let root = url?.standardizedFileURL
        guard root != rootURL else { return }
        rootURL = root
        guard let root, root != documentURL else {
            apply(engine.setRootFile(root?.path, scan: nil, nowMs: nowMs))
            return
        }
        Task { @MainActor [weak self] in
            guard let self else { return }
            let scan = await scanFile(root)
            guard rootURL == root else { return }
            apply(engine.setRootFile(root.path, scan: scan, nowMs: nowMs))
        }
    }

    /// The project's build command: its engine and shell-escape policy are
    /// the only ones the exact preview may use.
    func setBuildCommand(_ command: String) {
        let profile = ExactEquationProfile.resolve(buildCommand: command)
        exactProfile = profile
        apply(engine.setExactProfile(profile?.identity, nowMs: nowMs))
    }

    func requestExact() {
        apply(engine.requestExact(nowMs: nowMs))
    }

    /// A save or app activation: re-read the other files the context uses.
    func refreshExternalFiles() {
        let ids = engine.externalFileIDs
        guard !ids.isEmpty else { return }
        let stamps = externalStamps
        Task { @MainActor [weak self] in
            let changed: [(String, Date?, MathSourceScan?)] = await Task.detached(priority: .utility) {
                ids.compactMap { id in
                    let stamp = (try? FileManager.default.attributesOfItem(atPath: id))?[.modificationDate] as? Date
                    guard stamp != stamps[id] else { return nil }
                    // Same 4MiB ceiling as `scanFile` — a giant generated
                    // include must not stall the main actor's scan.
                    let size = (try? FileManager.default.attributesOfItem(atPath: id))?[.size] as? NSNumber
                    let text = (stamp == nil || (size?.intValue ?? .max) > 4 * 1024 * 1024)
                        ? nil : try? String(contentsOfFile: id, encoding: .utf8)
                    return (id, stamp, text.map { MathSourceScanner.scan($0) })
                }
            }.value
            guard let self else { return }
            for (id, stamp, diskScan) in changed {
                externalStamps[id] = stamp
                // Open buffers keep their unsaved text.
                let scan = await workspace.openText(URL(fileURLWithPath: id)).map { MathSourceScanner.scan($0) } ?? diskScan
                apply(engine.externalFileChanged(id, scan: scan, nowMs: nowMs))
            }
        }
    }

    func shutdown() {
        detach()
        wakeTask?.cancel()
        exactTasks.values.forEach { $0.cancel() }
        exactTasks.removeAll()
        exactTaskIDs.removeAll()
        cancellables.removeAll()
        if let token = workspaceToken { ExactEquationRenderer.removeWorkspaceArtifacts(token: token) }
    }

    // MARK: Editor binding

    func attach(textView: NSTextView, scrollView: NSScrollView) {
        guard self.textView !== textView else { return }
        detach()
        self.textView = textView
        let center = NotificationCenter.default
        if let storage = textView.textStorage {
            observers.append(center.addObserver(forName: NSTextStorage.didProcessEditingNotification, object: storage, queue: .main) { [weak self] notification in
                // Read on the posting thread: `Notification` is not Sendable,
                // so only a Bool may cross into the isolated closure.
                let editedCharacters = (notification.object as? NSTextStorage)?
                    .editedMask.contains(.editedCharacters) == true
                MainActor.assumeIsolated {
                    // Attribute-only passes (highlighting) are not edits.
                    guard editedCharacters else { return }
                    self?.textDidChange()
                }
            })
        }
        observers.append(center.addObserver(forName: NSTextView.didChangeSelectionNotification, object: textView, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.selectionDidChange() }
        })
        scrollView.contentView.postsBoundsChangedNotifications = true
        observers.append(center.addObserver(forName: NSView.boundsDidChangeNotification, object: scrollView.contentView, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.reposition() }
        })
        textView.postsFrameChangedNotifications = true
        observers.append(center.addObserver(forName: NSView.frameDidChangeNotification, object: textView, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.reposition() }
        })
        for name in [NSWindow.didBecomeKeyNotification, NSWindow.didResignKeyNotification] {
            observers.append(center.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                MainActor.assumeIsolated { self?.refreshFocus() }
            })
        }
        hoverTracker = EquationHoverTracker(textView: textView) { [weak self] location in
            guard let self else { return }
            apply(engine.hover(location: location, nowMs: nowMs))
        }
        keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            // NSEvent is not Sendable: only scalars cross into the isolated
            // closure, exactly as the text-edit observer does.
            let isEscape = event.keyCode == 53
            let modifierClear = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
                .subtracting(.capsLock).isEmpty
            let windowID = event.window.map { ObjectIdentifier($0) }
            let consumed = MainActor.assumeIsolated {
                self?.handleEscape(isEscape: isEscape, modifierClear: modifierClear,
                                   windowID: windowID) ?? false
            }
            return consumed ? nil : event
        }
        _ = renderer
        // Automatic pushes must not replenish the crash budget; explicit
        // re-enable (off→on) is the only recovery path (B5).
        revision += 1
        apply(engine.openDocument(fileID: documentURL?.path, revision: revision, nowMs: nowMs))
        refreshFocus()
        selectionDidChange()
    }

    /// SwiftUI may dismantle the previous editor after mounting the next —
    /// only the view still bound here is let go.
    func detach(from textView: NSTextView) {
        guard self.textView === textView else { return }
        detach()
    }

    func detach() {
        observers.forEach { NotificationCenter.default.removeObserver($0) }
        observers.removeAll()
        if let keyMonitor { NSEvent.removeMonitor(keyMonitor) }
        keyMonitor = nil
        hoverTracker?.invalidate()
        hoverTracker = nil
        windowObservation = nil
        observedWindow = nil
        textView = nil
        presentation = nil
        showToken += 1
        popover.hide()
    }

    private func textDidChange() {
        revision += 1
        apply(engine.textChanged(revision: revision, nowMs: nowMs))
        apply(engine.compositionChanged(textView?.hasMarkedText() ?? false, nowMs: nowMs))
    }

    private func selectionDidChange() {
        guard let textView else { return }
        let range = textView.selectedRange()
        apply(engine.compositionChanged(textView.hasMarkedText(), nowMs: nowMs))
        apply(engine.selectionChanged(location: range.location, length: range.length, revision: revision, nowMs: nowMs))
        refreshFocus()
    }

    private func refreshFocus() {
        guard let textView else { return }
        if let window = textView.window, window !== observedWindow {
            observedWindow = window
            windowObservation = window.observe(\.firstResponder) { [weak self] _, _ in
                DispatchQueue.main.async { MainActor.assumeIsolated { self?.refreshFocus() } }
            }
        }
        let focused = textView.window?.isKeyWindow == true && textView.window?.firstResponder === textView
        apply(engine.focusChanged(focused, nowMs: nowMs))
    }

    /// Escape hides a visible or pending preview; other keys and IME
    /// composition pass. `consumed` is the engine's verdict, so find and
    /// completion still receive Escape when nothing preview-related is
    /// active (N5). Inputs are scalars pulled off the event before the hop.
    private func handleEscape(isEscape: Bool, modifierClear: Bool, windowID: ObjectIdentifier?) -> Bool {
        guard isEscape, modifierClear, let textView, let windowID,
              textView.window.map(ObjectIdentifier.init) == windowID,
              !textView.hasMarkedText() else { return false }
        let (consumed, commands) = engine.escape(nowMs: nowMs)
        apply(commands)
        return consumed
    }

    // MARK: Settings / appearance

    private var isTeXDocument: Bool {
        ["tex", "ltx", "sty", "cls"].contains(documentURL?.pathExtension.lowercased() ?? "")
    }

    private func refreshSettings() {
        let store = SettingsStore.shared
        // The crash budget recovers only when the user re-enables the
        // preview (persisted off→on); a document-type change must not
        // re-arm it even if that also flips the effective flag (P3).
        let persistedEnabled = store.equationPreviewEnabled
        let settings = EquationPreviewSettings(
            enabled: persistedEnabled && isTeXDocument,
            whileTyping: store.equationPreviewWhileTyping,
            placement: store.equationPreviewPlacement,
            renderer: store.equationPreviewRenderer,
            delayMilliseconds: store.equationPreviewDelayMilliseconds
        )
        if persistedEnabled, lastEnabled == false {
            renderer.recover()
        }
        lastEnabled = persistedEnabled
        apply(engine.setSettings(settings, nowMs: nowMs))
    }

    private func refreshAppearance() {
        let dark = AppearanceSettings.shared.effectiveDark
        let scheme: EquationPreviewScheme = NSWorkspace.shared.accessibilityDisplayShouldIncreaseContrast
            ? (dark ? .highContrastDark : .highContrastLight)
            : (dark ? .dark : .light)
        // Math reads best a little larger than body text.
        let appearance = EquationPreviewAppearance(scheme: scheme, fontSize: AppearanceSettings.shared.fontSize + 3)
        apply(engine.setAppearance(appearance, nowMs: nowMs))
    }

    /// Foreground/background of the popover body under the current scheme.
    private var colors: (foreground: NSColor, background: NSColor) {
        switch engine.appearance.scheme {
        case .highContrastLight: return (.black, .white)
        case .highContrastDark: return (.white, .black)
        case .dark: return (NSColor(white: 0.92, alpha: 1), NSColor(white: 0.16, alpha: 1))
        case .light: return (NSColor(white: 0.1, alpha: 1), NSColor(white: 0.99, alpha: 1))
        }
    }

    // MARK: Commands

    private func apply(_ commands: [EquationPreviewCommand]) {
        for command in commands {
            switch command {
            case let .render(request): startRender(request)
            case let .renderExact(request): startExact(request)
            case let .cancelExact(key): exactTasks[key]?.cancel()
            case let .show(presentation): show(presentation)
            case .hide:
                presentation = nil
                showToken += 1
                popover.hide()
            case let .resolveIncludes(requests): resolve(requests)
            }
        }
        armWake()
    }

    private func armWake() {
        wakeTask?.cancel()
        guard let deadline = engine.nextDeadline else { return }
        let now = nowMs
        let delay = deadline > now ? deadline - now : 0
        wakeTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(Int(delay)))
            guard !Task.isCancelled, let self else { return }
            let text = textView?.string ?? ""
            apply(engine.poll(nowMs: max(nowMs, deadline), text: text))
        }
    }

    private func startRender(_ request: EquationRenderRequest) {
        Task { @MainActor [weak self] in
            guard let self else { return }
            let outcome = await renderer.render(request)
            apply(engine.fastRenderCompleted(key: request.key, outcome: outcome, nowMs: nowMs))
        }
    }

    private var workspaceToken: String? {
        workspace.projectRoot.map { MathPreviewHash.hex(MathPreviewHash.combine(MathPreviewHash.basis, $0.standardizedFileURL.path)) }
    }

    private func startExact(_ request: ExactEquationRequest) {
        guard let exactProfile, let token = workspaceToken,
              let directory = (rootURL ?? documentURL)?.deletingLastPathComponent() else {
            apply(engine.exactRenderCompleted(key: request.key, outcome: .failed, nowMs: nowMs))
            return
        }
        // Identity guard: a canceled job finishing late under the same key
        // must not remove or answer the job that replaced it.
        let jobID = UUID()
        exactTaskIDs[request.key] = jobID
        exactTasks[request.key] = Task { @MainActor [weak self] in
            var environment: [String: String] = [:]
            if case let .inherit(overrides) = await StreamingBuildExecutor.buildEnvironment(.inherit(overrides: [:])) {
                environment = overrides
            }
            let renderer = ExactEquationRenderer(profile: exactProfile, mainDirectory: directory,
                                                 environment: environment, workspaceToken: token)
            // Nonisolated async: compiles off the main actor; cancelling this
            // task terminates the engine's process group.
            let pdf = try? await renderer.render(document: request.document)
            guard let self, exactTaskIDs[request.key] == jobID else { return }
            exactTasks[request.key] = nil
            exactTaskIDs[request.key] = nil
            apply(engine.exactRenderCompleted(key: request.key, outcome: pdf.map(ExactEquationOutcome.pdf) ?? .failed, nowMs: nowMs))
        }
    }

    private func resolve(_ requests: [MathIncludeRequest]) {
        Task { @MainActor [weak self] in
            guard let self else { return }
            var results: [(key: MathIncludeKey, fileID: String?)] = []
            var scans: [String: MathSourceScan] = [:]
            for request in requests {
                let candidates = request.candidates
                let found: String? = await Task.detached(priority: .utility) {
                    candidates.first { FileManager.default.isReadableFile(atPath: $0) }
                }.value
                results.append((request.key, found))
                if let found, scans[found] == nil, found != documentURL?.path,
                   let scan = await scanFile(URL(fileURLWithPath: found)) {
                    scans[found] = scan
                }
            }
            apply(engine.includesResolved(results, scans: scans, nowMs: nowMs))
        }
    }

    /// Open-buffer text first, else disk (off the main actor, bounded).
    private func scanFile(_ url: URL) async -> MathSourceScan? {
        if let text = await workspace.openText(url) { return MathSourceScanner.scan(text) }
        let path = url.path
        let (stamp, scan) = await Task.detached(priority: .utility) { () -> (Date?, MathSourceScan?) in
            let attributes = try? FileManager.default.attributesOfItem(atPath: path)
            guard let size = (attributes?[.size] as? NSNumber)?.intValue, size <= 4 * 1024 * 1024,
                  let text = try? String(contentsOfFile: path, encoding: .utf8) else { return (nil, nil) }
            return (attributes?[.modificationDate] as? Date, MathSourceScanner.scan(text))
        }.value
        externalStamps[path] = stamp
        return scan
    }

    // MARK: Presentation

    private func show(_ presentation: EquationPreviewPresentation) {
        self.presentation = presentation
        showToken += 1
        let token = showToken
        let (foreground, background) = colors
        switch presentation.content {
        case let .fast(svg):
            Task { @MainActor [weak self] in
                guard let self else { return }
                let size = await renderer.show(
                    svg: svg, foreground: foreground.hexString ?? "#000000",
                    background: background.hexString ?? "#ffffff", fontSize: engine.appearance.fontSize
                )
                guard token == showToken else { return }
                guard let size else { popover.hide(); return }
                present(.fast(size: size), label: String(localized: "equation_preview.fast_badge"), background: background)
            }
        case let .exact(pdf):
            let scale = textView?.window?.backingScaleFactor ?? 2
            // TeX typesets at 10pt: magnify to the fast preview's size.
            let zoom = CGFloat(engine.appearance.fontSize / 10)
            Task { @MainActor [weak self] in
                let cropped = await Task.detached(priority: .userInitiated) {
                    ExactEquationImage.cropped(pdf: pdf, scale: scale * zoom).map(CroppedImage.init)
                }.value
                guard let self, token == showToken else { return }
                guard let image = cropped?.image else {
                    present(.message(String(localized: "equation_preview.unavailable")), label: "", background: background)
                    return
                }
                // TeX output keeps its own colors on paper white.
                present(.exact(image, scale: scale), label: String(localized: "equation_preview.exact_badge"), background: .white)
            }
        case let .unavailable(reason):
            let text = reason == .exactUnavailable
                ? String(localized: "equation_preview.exact_unavailable")
                : String(localized: "equation_preview.unavailable")
            present(.message(text), label: "", background: background)
        }
    }

    private func present(_ body: EquationPreviewPopover.Body, label: String, background: NSColor) {
        guard let presentation, let textView, let rect = anchorRect(for: presentation) else {
            popover.hide()
            return
        }
        popover.present(body, label: label, source: sourceText(presentation), rect: rect, in: textView,
                        placement: presentation.placement, background: background)
    }

    private func clampedRange(_ presentation: EquationPreviewPresentation) -> NSRange? {
        guard let textView else { return nil }
        let length = (textView.string as NSString).length
        let lower = min(presentation.anchor.lowerBound, length)
        let upper = min(presentation.anchor.upperBound, length)
        return NSRange(location: lower, length: upper - lower)
    }

    private func sourceText(_ presentation: EquationPreviewPresentation) -> String {
        guard let textView, let range = clampedRange(presentation) else { return "" }
        return (textView.string as NSString).substring(with: range)
    }

    /// The visible part of the region in text-view coordinates, or nil when
    /// it scrolled out of view.
    private func anchorRect(for presentation: EquationPreviewPresentation) -> NSRect? {
        guard let textView, let range = clampedRange(presentation),
              let layoutManager = textView.layoutManager, let container = textView.textContainer else { return nil }
        let glyphs = layoutManager.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
        var rect = layoutManager.boundingRect(forGlyphRange: glyphs, in: container)
        rect.origin.x += textView.textContainerOrigin.x
        rect.origin.y += textView.textContainerOrigin.y
        let visible = rect.intersection(textView.visibleRect)
        return visible.isNull || visible.isEmpty ? nil : visible
    }

    private func reposition() {
        guard let presentation else { return }
        if let rect = anchorRect(for: presentation) {
            if popover.isShown {
                popover.reposition(rect)
            } else {
                show(presentation) // scrolled back into view
            }
        } else {
            popover.hide()
        }
    }
}

/// Pointer tracking over the text view: the character under the pointer,
/// or nil over empty space, outside the text or while a button is held.
@MainActor
private final class EquationHoverTracker: NSResponder {
    private weak var textView: NSTextView?
    private var area: NSTrackingArea?
    private let report: (Int?) -> Void

    init(textView: NSTextView, report: @escaping (Int?) -> Void) {
        self.textView = textView
        self.report = report
        super.init()
        let area = NSTrackingArea(rect: .zero, options: [.mouseMoved, .mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect],
                                  owner: self, userInfo: nil)
        textView.addTrackingArea(area)
        self.area = area
    }

    required init?(coder: NSCoder) { nil }

    func invalidate() {
        if let area { textView?.removeTrackingArea(area) }
        area = nil
    }

    override func mouseMoved(with event: NSEvent) { report(character(at: event)) }
    override func mouseEntered(with event: NSEvent) { report(character(at: event)) }
    override func mouseExited(with event: NSEvent) { report(nil) }

    private func character(at event: NSEvent) -> Int? {
        guard NSEvent.pressedMouseButtons == 0, let textView,
              let layoutManager = textView.layoutManager, let container = textView.textContainer,
              layoutManager.numberOfGlyphs > 0 else { return nil }
        let point = textView.convert(event.locationInWindow, from: nil)
        let local = NSPoint(x: point.x - textView.textContainerOrigin.x, y: point.y - textView.textContainerOrigin.y)
        let glyph = layoutManager.glyphIndex(for: local, in: container)
        guard glyph < layoutManager.numberOfGlyphs,
              layoutManager.boundingRect(forGlyphRange: NSRange(location: glyph, length: 1), in: container).contains(local)
        else { return nil }
        return layoutManager.characterIndexForGlyph(at: glyph)
    }
}
