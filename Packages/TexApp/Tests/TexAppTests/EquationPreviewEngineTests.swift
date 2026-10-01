import EditorFeature
import LanguageCore
import Foundation
import XCTest

/// Drives `EquationPreviewEngine` the way a host does: events with a
/// virtual clock, `poll` at `nextDeadline`, renderer completions fed back
/// with artificial delays. The Rust port runs the same scenarios.
final class EquationPreviewEngineTests: XCTestCase {
    private struct Host {
        var engine: EquationPreviewEngine
        var text = ""
        var now: UInt64 = 1_000
        var revision: UInt64 = 0
        var log: [EquationPreviewCommand] = []
        /// The host's include resolution: unknown targets are missing files,
        /// unless a test answers them itself.
        var answersIncludesAsMissing = true

        init(settings: EquationPreviewSettings = EquationPreviewSettings()) {
            engine = EquationPreviewEngine(settings: settings)
        }

        mutating func record(_ commands: [EquationPreviewCommand]) {
            log += commands
            guard answersIncludesAsMissing else { return }
            for case let .resolveIncludes(requests) in commands {
                record(engine.includesResolved(requests.map { ($0.key, nil) }, scans: [:], nowMs: now))
            }
        }

        /// Opens `marked` (‸ = caret) as the focused document.
        mutating func open(_ marked: String, fileID: String = "/p/main.tex", ready: Bool = true) {
            let (source, caret) = Self.split(marked)
            text = source
            revision += 1
            record(engine.openDocument(fileID: fileID, revision: revision, nowMs: now))
            record(engine.focusChanged(true, nowMs: now))
            if ready { record(engine.rendererReady(version: "test-renderer", nowMs: now)) }
            if let caret { record(engine.selectionChanged(location: caret, length: 0, revision: revision, nowMs: now)) }
        }

        /// An edit: new text and its post-edit caret at the new revision.
        mutating func edit(_ marked: String) {
            let (source, caret) = Self.split(marked)
            text = source
            revision += 1
            if let caret { record(engine.selectionChanged(location: caret, length: 0, revision: revision, nowMs: now)) }
            record(engine.textChanged(revision: revision, nowMs: now))
        }

        mutating func moveCaret(_ location: Int, length: Int = 0) {
            record(engine.selectionChanged(location: location, length: length, revision: revision, nowMs: now))
        }

        /// Advances the clock, polling at every deadline on the way.
        mutating func advance(_ milliseconds: UInt64) {
            let end = now + milliseconds
            while let deadline = engine.nextDeadline, deadline <= end {
                now = max(now, deadline)
                let current = text
                record(engine.poll(nowMs: now, text: current))
            }
            now = end
        }

        var renders: [EquationRenderRequest] {
            log.compactMap { if case let .render(request) = $0 { request } else { nil } }
        }

        var exactRenders: [ExactEquationRequest] {
            log.compactMap { if case let .renderExact(request) = $0 { request } else { nil } }
        }

        var shows: [EquationPreviewPresentation] {
            log.compactMap { if case let .show(presentation) = $0 { presentation } else { nil } }
        }

        /// Visible state after replaying show/hide.
        var visible: EquationPreviewPresentation? {
            var current: EquationPreviewPresentation?
            for command in log {
                if case let .show(presentation) = command { current = presentation }
                if case .hide = command { current = nil }
            }
            return current
        }

        mutating func complete(_ request: EquationRenderRequest, _ outcome: EquationRenderOutcome) {
            record(engine.fastRenderCompleted(key: request.key, outcome: outcome, nowMs: now))
        }

        mutating func completeLatest(svg: String, file: StaticString = #filePath, line: UInt = #line) {
            guard let request = renders.last else {
                XCTFail("no render request to complete", file: file, line: line)
                return
            }
            complete(request, .svg(svg))
        }

        static func split(_ marked: String) -> (String, Int?) {
            guard let range = marked.range(of: "\u{2038}") else { return (marked, nil) }
            let offset = marked.utf16.distance(from: marked.startIndex, to: range.lowerBound)
            return (marked.replacingCharacters(in: range, with: ""), offset)
        }
    }

    private func svg(_ request: EquationRenderRequest?) -> String {
        "<svg data-source=\"\(request?.source ?? "")\"/>"
    }

    func testCaretEnteringMathRendersAndLeavingHides() {
        var host = Host()
        host.open("Text $a‸+b$ more.")
        host.advance(100)
        XCTAssertEqual(host.renders.map(\.source), ["a+b"])
        XCTAssertEqual(host.renders.first?.displayMode, false)
        host.completeLatest(svg: "<svg id='ab'/>")
        XCTAssertEqual(host.visible?.content, .fast(svg: "<svg id='ab'/>"))
        XCTAssertEqual(host.visible?.anchor, 5..<10)
        XCTAssertEqual(host.visible?.trigger, .caret)

        host.moveCaret(2)
        host.advance(100)
        XCTAssertNil(host.visible)
    }

    func testDisplayModeDistinguishesDelimiters() {
        var host = Host()
        host.open("\\[x‸^2\\]")
        host.advance(100)
        XCTAssertEqual(host.renders.last?.displayMode, true)
        XCTAssertEqual(host.renders.last?.source, "x^2")
    }

    func testHoverPreviewsAnotherEquationWithoutMovingCaret() {
        var host = Host()
        let marked = "$a‸$ and $b$"
        host.open(marked)
        host.advance(100)
        host.completeLatest(svg: "A")
        XCTAssertEqual(host.visible?.content, .fast(svg: "A"))

        let bOffset = (host.text as NSString).range(of: "b").location
        host.record(host.engine.hover(location: bOffset, nowMs: host.now))
        host.advance(100)
        XCTAssertEqual(host.renders.last?.source, "b")
        host.completeLatest(svg: "B")
        XCTAssertEqual(host.visible?.content, .fast(svg: "B"))
        XCTAssertEqual(host.visible?.trigger, .hover)

        // Pointer leaves: a grace period, then the caret equation returns.
        host.record(host.engine.hover(location: nil, nowMs: host.now))
        host.advance(100)
        XCTAssertEqual(host.visible?.content, .fast(svg: "B"))
        host.advance(300)
        XCTAssertEqual(host.visible?.content, .fast(svg: "A"))
        XCTAssertEqual(host.visible?.trigger, .caret)
    }

    func testHoverSurvivesTheMoveOntoThePopover() {
        var host = Host()
        host.open("Intro text\n$x^2$ tail")
        let x = (host.text as NSString).range(of: "x^").location
        host.record(host.engine.hover(location: x, nowMs: host.now))
        host.advance(100)
        host.completeLatest(svg: "X")
        // Crossing plain text, then entering the popover.
        host.record(host.engine.hover(location: 2, nowMs: host.now))
        host.advance(100)
        host.record(host.engine.pointerInPopover(true, nowMs: host.now))
        host.advance(1_000)
        XCTAssertEqual(host.visible?.content, .fast(svg: "X"))
        host.record(host.engine.pointerInPopover(false, nowMs: host.now))
        host.record(host.engine.hover(location: nil, nowMs: host.now))
        host.advance(400)
        XCTAssertNil(host.visible)
    }

    /// Spec 26.4: a → ab → abc → abcd with a slow renderer; only abcd may
    /// ever become visible.
    func testRapidEditsPublishOnlyTheNewestResult() {
        var host = Host()
        host.open("$a‸$")
        host.advance(100)
        let first = host.renders[0]
        for marked in ["$ab‸$", "$abc‸$", "$abcd‸$"] {
            host.edit(marked)
            host.advance(90)
        }
        // Still only the first request is in flight; later ones coalesce.
        XCTAssertEqual(host.renders.map(\.source), ["a"])
        host.complete(first, .svg(svg(first)))
        XCTAssertTrue(host.shows.isEmpty)
        XCTAssertEqual(host.renders.map(\.source), ["a", "abcd"])
        host.complete(host.renders[1], .svg(svg(host.renders[1])))
        XCTAssertEqual(host.shows.map(\.content), [.fast(svg: svg(host.renders[1]))])
    }

    func testLateResultAfterAnEditIsNotShown() {
        var host = Host()
        host.open("$x‸$")
        host.advance(100)
        let request = host.renders[0]
        host.edit("$x+‸$")
        host.complete(request, .svg("stale"))
        XCTAssertNil(host.visible)
        // The newer revision renders on its own.
        host.advance(100)
        XCTAssertEqual(host.renders.last?.source, "x+")
    }

    func testCaretMovesInsideOneEquationReuseTheCache() {
        var host = Host()
        host.open("$a‸bc$")
        host.advance(100)
        host.completeLatest(svg: "ABC")
        host.moveCaret(3)
        host.advance(100)
        host.moveCaret(4)
        host.advance(100)
        XCTAssertEqual(host.renders.count, 1)
        XCTAssertEqual(host.visible?.content, .fast(svg: "ABC"))
    }

    /// Spec 13: an invalid intermediate keeps the last preview ~300 ms, then hides.
    func testInvalidIntermediateKeepsLastGoodBrieflyThenHides() {
        var host = Host()
        host.open("$\\frac{1}{2}‸$")
        host.advance(100)
        host.completeLatest(svg: "HALF")
        host.edit("$\\frac{1}{2}{‸$")
        host.advance(100)
        XCTAssertEqual(host.visible?.content, .fast(svg: "HALF"))
        host.advance(300)
        XCTAssertNil(host.visible)
        XCTAssertFalse(host.log.contains { if case .renderExact = $0 { true } else { false } })
    }

    /// Spec 26.3: every intermediate state is handled without a crash,
    /// modal error or stale overwrite.
    func testIncompleteEditingSequence() {
        var host = Host()
        host.open("")
        var answered = 0
        func answer(_ host: inout Host) {
            while answered < host.renders.count {
                let request = host.renders[answered]
                answered += 1
                host.complete(request, request.source == "\\frac{1}{2}" ? .svg("HALF") : .failed(.invalid))
            }
        }
        for marked in ["$‸", "$\\frac‸", "$\\frac{‸", "$\\frac{1}‸", "$\\frac{1}{‸"] {
            host.edit(marked)
            host.advance(90)
            answer(&host)
            XCTAssertNil(host.visible, marked)
        }
        host.edit("$\\frac{1}{2}‸$")
        host.advance(90)
        answer(&host)
        XCTAssertEqual(host.visible?.content, .fast(svg: "HALF"))
        XCTAssertEqual(host.shows.count, 1)
    }

    func testRedefinitionChangesTheCacheKey() {
        var host = Host()
        host.open("\\newcommand{\\R}{\\mathbb{R}}\n$x \\in \\R‸$")
        host.advance(100)
        let first = host.renders[0]
        XCTAssertEqual(first.definitions, ["\\newcommand{\\R}{\\mathbb{R}}"])
        host.complete(first, .svg("BB"))

        host.edit("\\renewcommand{\\R}{\\mathbf{R}}\n$x \\in \\R‸$")
        host.advance(100)
        let second = host.renders[1]
        XCTAssertEqual(second.definitions, ["\\renewcommand{\\R}{\\mathbf{R}}"])
        XCTAssertNotEqual(first.contextKey, second.contextKey)
        host.complete(second, .svg("BF"))
        XCTAssertEqual(host.visible?.content, .fast(svg: "BF"))

        // Back to the original definition: served from the cache.
        host.edit("\\newcommand{\\R}{\\mathbb{R}}\n$x \\in \\R‸$")
        host.advance(100)
        XCTAssertEqual(host.renders.count, 2)
        XCTAssertEqual(host.visible?.content, .fast(svg: "BB"))
    }

    /// State isolation across documents: a definition in document A must not
    /// reach document B's request.
    func testDefinitionsDoNotLeakAcrossDocuments() {
        var host = Host()
        host.open("\\newcommand{\\R}{\\mathbb{R}}\n$\\R‸$", fileID: "/a/a.tex")
        host.advance(100)
        let fromA = host.renders[0]
        host.complete(fromA, .svg("A"))
        host.open("$\\R‸$", fileID: "/b/b.tex")
        host.advance(100)
        let fromB = host.renders[1]
        XCTAssertEqual(fromB.definitions, [])
        XCTAssertNotEqual(fromA.contextKey, fromB.contextKey)
        XCTAssertNotEqual(fromA.key, fromB.key)
    }

    func testIncludedDefinitionsResolveThroughTheHost() throws {
        var host = Host()
        host.answersIncludesAsMissing = false
        host.open("\\input{commands}\n$x \\in \\R‸$")
        host.advance(100)
        let requests = host.log.compactMap { if case let .resolveIncludes(r) = $0 { r } else { nil } }.flatMap { $0 }
        XCTAssertEqual(requests.map(\.candidates), [["/p/commands.tex"]])
        // No render with a known-incomplete context (it would flash "unavailable").
        XCTAssertTrue(host.renders.isEmpty)
        XCTAssertNil(host.visible)
        let scan = MathSourceScanner.scan("\\newcommand{\\R}{\\mathbb{R}}\n")
        let request = try XCTUnwrap(requests.first)
        host.record(host.engine.includesResolved([(request.key, "/p/commands.tex")], scans: ["/p/commands.tex": scan], nowMs: host.now))
        host.advance(10)
        XCTAssertEqual(host.renders.map(\.definitions), [["\\newcommand{\\R}{\\mathbb{R}}"]])
    }

    func testEscapeHidesUntilAnotherEquation() {
        var host = Host()
        host.open("$a‸$ and $b$")
        host.advance(100)
        host.completeLatest(svg: "A")
        let escape = host.engine.escape(nowMs: host.now)
        host.record(escape.commands)
        XCTAssertTrue(escape.consumed)
        XCTAssertNil(host.visible)
        host.moveCaret(1)
        host.advance(100)
        XCTAssertNil(host.visible)
        let b = (host.text as NSString).range(of: "b").location
        host.moveCaret(b)
        host.advance(100)
        XCTAssertEqual(host.renders.last?.source, "b")
        host.completeLatest(svg: "B")
        XCTAssertEqual(host.visible?.content, .fast(svg: "B"))
    }

    func testFocusLossCompositionSelectionAndDisableHide() {
        var host = Host()
        host.open("$a‸$")
        host.advance(100)
        host.completeLatest(svg: "A")
        host.record(host.engine.focusChanged(false, nowMs: host.now))
        XCTAssertNil(host.visible)
        host.record(host.engine.focusChanged(true, nowMs: host.now))
        host.advance(100)
        XCTAssertEqual(host.visible?.content, .fast(svg: "A"))

        host.record(host.engine.compositionChanged(true, nowMs: host.now))
        XCTAssertNil(host.visible)
        host.advance(500)
        XCTAssertNil(host.visible)
        host.record(host.engine.compositionChanged(false, nowMs: host.now))
        host.advance(100)
        XCTAssertEqual(host.visible?.content, .fast(svg: "A"))

        host.moveCaret(1, length: 1)
        host.advance(100)
        XCTAssertNil(host.visible)
        host.moveCaret(1)
        host.advance(100)
        XCTAssertEqual(host.visible?.content, .fast(svg: "A"))

        host.record(host.engine.setSettings(EquationPreviewSettings(enabled: false), nowMs: host.now))
        XCTAssertNil(host.visible)
        host.moveCaret(2)
        host.advance(500)
        XCTAssertNil(host.visible)
        XCTAssertEqual(host.renders.count, 1)
    }

    func testPreviewWhileTypingOffWaitsForNavigation() {
        var host = Host(settings: EquationPreviewSettings(whileTyping: false))
        host.open("$a‸$")
        host.advance(100)
        host.completeLatest(svg: "A")
        host.edit("$ab‸$")
        XCTAssertNil(host.visible)
        host.advance(500)
        XCTAssertNil(host.visible)
        XCTAssertEqual(host.renders.count, 1)
        host.moveCaret(2)
        host.advance(100)
        XCTAssertEqual(host.renders.last?.source, "ab")
    }

    func testDelaySettingControlsTheTypingDebounce() {
        var host = Host(settings: EquationPreviewSettings(delayMilliseconds: 150))
        host.open("")
        host.advance(40)
        host.edit("$ab‸$")
        host.advance(149)
        XCTAssertTrue(host.renders.isEmpty)
        host.advance(1)
        XCTAssertEqual(host.renders.map(\.source), ["ab"])

        var instant = Host(settings: EquationPreviewSettings(delayMilliseconds: 0))
        instant.open("$a‸$")
        instant.advance(0)
        XCTAssertEqual(instant.renders.map(\.source), ["a"])
    }

    func testUnknownCommandIsUnavailableInFastMode() {
        var host = Host()
        host.open("$\\mySpecialOperator{x}‸$")
        host.advance(100)
        host.complete(host.renders[0], .failed(.undefinedCommand))
        XCTAssertEqual(host.visible?.content, .unavailable(.unsupported))
        host.advance(5_000)
        XCTAssertTrue(host.exactRenders.isEmpty)
    }

    /// The exact TeX fallback waits for the equation to settle — never
    /// once per keystroke — and follows the latest equation only.
    func testExactFallbackRunsOnlyAfterSettling() throws {
        var host = Host(settings: EquationPreviewSettings(renderer: .fastWithTeXFallback))
        host.record(host.engine.setExactProfile("pdflatex|projectDefault", nowMs: host.now))
        host.open("\\documentclass{article}\n\\usepackage{mine}\n\\begin{document}\n$\\mine{a}‸$\n\\end{document}")
        for marked in ["$\\mine{ab}‸$", "$\\mine{abc}‸$"] {
            host.advance(100)
            if let request = host.renders.last { host.complete(request, .failed(.undefinedCommand)) }
            host.edit("\\documentclass{article}\n\\usepackage{mine}\n\\begin{document}\n" + marked + "\n\\end{document}")
        }
        host.advance(100)
        host.complete(try XCTUnwrap(host.renders.last), .failed(.undefinedCommand))
        XCTAssertTrue(host.exactRenders.isEmpty)
        host.advance(700)
        let exact = try XCTUnwrap(host.exactRenders.first)
        XCTAssertEqual(host.exactRenders.count, 1)
        XCTAssertTrue(exact.document.hasPrefix("\\documentclass{article}\n\\usepackage{mine}\n"))
        XCTAssertTrue(exact.document.contains("$\\mine{abc}$"))
        host.record(host.engine.exactRenderCompleted(key: exact.key, outcome: .pdf([1, 2, 3]), nowMs: host.now))
        XCTAssertEqual(host.visible?.content, .exact(pdf: [1, 2, 3]))
    }

    func testExactJobIsCancelledWhenTheEquationChanges() throws {
        var host = Host(settings: EquationPreviewSettings(renderer: .fastWithTeXFallback))
        host.record(host.engine.setExactProfile("xelatex|disabled", nowMs: host.now))
        host.open("$\\foo‸$ and $b$")
        host.advance(100)
        host.complete(host.renders[0], .failed(.undefinedCommand))
        host.advance(700)
        let exact = try XCTUnwrap(host.exactRenders.first)
        let b = (host.text as NSString).range(of: "b").location
        host.moveCaret(b)
        host.advance(100)
        XCTAssertTrue(host.log.contains(.cancelExact(key: exact.key)))
        host.record(host.engine.exactRenderCompleted(key: exact.key, outcome: .pdf([9]), nowMs: host.now))
        XCTAssertNotEqual(host.visible?.content, .exact(pdf: [9]))
    }

    func testFallbackWithoutSupportedEngineSaysSo() {
        var host = Host(settings: EquationPreviewSettings(renderer: .fastWithTeXFallback))
        host.open("$\\foo‸$")
        host.advance(100)
        host.complete(host.renders[0], .failed(.undefinedCommand))
        XCTAssertEqual(host.visible?.content, .unavailable(.exactUnavailable))
    }

    func testExplicitExactRequestWorksInFastMode() throws {
        var host = Host()
        host.record(host.engine.setExactProfile("lualatex|projectDefault", nowMs: host.now))
        host.open("$x^2‸$")
        host.advance(100)
        host.completeLatest(svg: "X2")
        host.record(host.engine.requestExact(nowMs: host.now))
        host.advance(1)
        let exact = try XCTUnwrap(host.exactRenders.first)
        XCTAssertTrue(exact.document.contains("\\documentclass{article}"))
        host.record(host.engine.exactRenderCompleted(key: exact.key, outcome: .pdf([7]), nowMs: host.now))
        XCTAssertEqual(host.visible?.content, .exact(pdf: [7]))
    }

    func testThemeChangeRepresentsWithoutRerender() {
        // C10: scheme is applied at show-time — a theme change re-presents
        // the cached SVG instead of paying a second typeset.
        var host = Host()
        host.open("$a‸$")
        host.advance(100)
        host.completeLatest(svg: "light")
        host.record(host.engine.setAppearance(EquationPreviewAppearance(scheme: .dark, fontSize: 13), nowMs: host.now))
        host.advance(1)
        XCTAssertEqual(host.renders.count, 1)
        XCTAssertEqual(host.visible?.content, .fast(svg: "light"))
    }

    func testFontSizeChangeRerenders() {
        // fontSize is in the cache key: it changes em/ex metrics.
        var host = Host()
        host.open("$a‸$")
        host.advance(100)
        host.completeLatest(svg: "light")
        host.record(host.engine.setAppearance(EquationPreviewAppearance(scheme: .light, fontSize: 14), nowMs: host.now))
        host.advance(1)
        XCTAssertEqual(host.renders.count, 2)
        XCTAssertEqual(host.renders[1].key.fontSize, 14)
    }

    func testRendererVersionChangeDropsTheCache() {
        var host = Host()
        host.open("$a‸$")
        host.advance(100)
        host.completeLatest(svg: "v1")
        host.record(host.engine.rendererReady(version: "other", nowMs: host.now))
        host.advance(1)
        XCTAssertEqual(host.renders.count, 2)
    }

    /// Spec 26.5: dozens of aligned rows go to the renderer whole; the size
    /// bound turns only pathological sources into "unavailable".
    func testLargeAlignedExpressionAndBounds() {
        let rows = (1...60).map { "a_{\($0)} &= b_{\($0)} + c_{\($0)} \\\\" }.joined(separator: "\n")
        var host = Host()
        host.open("\\begin{aligned}\n" + rows + "‸\n\\end{aligned}")
        host.advance(100)
        XCTAssertEqual(host.renders.last?.source, "\\begin{aligned}\n" + rows + "\n\\end{aligned}")
        XCTAssertEqual(host.renders.last?.displayMode, true)

        var huge = Host()
        huge.open("$" + String(repeating: "x+", count: EquationPreviewEngine.maximumSourceBytes) + "‸$")
        huge.advance(100)
        XCTAssertTrue(huge.renders.isEmpty)
        XCTAssertEqual(huge.visible?.content, .unavailable(.tooLarge))
    }

    func testCacheIsBoundedAndEvictsLeastRecentlyUsed() {
        var host = Host()
        host.open("$x0‸$")
        for index in 0...EquationPreviewEngine.cacheCapacity {
            host.edit("$x\(index)‸$")
            host.advance(100)
            host.completeLatest(svg: "s\(index)")
        }
        let before = host.renders.count
        host.edit("$x0‸$")
        host.advance(100)
        XCTAssertEqual(host.renders.count, before + 1, "x0 was evicted")
        let last = EquationPreviewEngine.cacheCapacity
        host.edit("$x\(last)‸$")
        host.advance(100)
        XCTAssertEqual(host.renders.count, before + 1, "recent entry still cached")
    }

    func testRendererNotReadyDefersRequests() {
        var host = Host()
        host.open("$a‸$", ready: false)
        host.advance(100)
        XCTAssertTrue(host.renders.isEmpty)
        host.record(host.engine.rendererReady(version: "late", nowMs: host.now))
        host.advance(1)
        XCTAssertEqual(host.renders.map(\.source), ["a"])
    }

    /// B3: pointer parked over B while typing edits A — the stale hover
    /// anchor must be dropped, and the caret's equation takes over.
    func testTypingSupersedesAStaleHoverTarget() {
        var host = Host()
        host.open("$a‸$ and $b$")
        host.advance(100)
        host.completeLatest(svg: "A")
        let bOffset = host.text.distance(from: host.text.startIndex, to: host.text.firstIndex(of: "b")!)
        host.record(host.engine.hover(location: bOffset, nowMs: host.now))
        host.advance(100)
        host.completeLatest(svg: "B")
        XCTAssertEqual(host.visible?.trigger, .hover)

        host.edit("$ax‸$ and $b$")
        host.advance(100)
        XCTAssertEqual(host.renders.last?.source, "ax")
        host.completeLatest(svg: "AX")
        XCTAssertEqual(host.visible?.content, .fast(svg: "AX"))
        XCTAssertEqual(host.visible?.trigger, .caret)
    }

    /// C6 r2: grace expires while the pointer already rests on equation C —
    /// expiry must clear the hover TARGET, not the location.
    func testGraceExpiryWithPointerOnNewEquationShowsIt() {
        var host = Host()
        host.open("$a‸$ x $b$ x $c$")
        host.advance(100)
        host.completeLatest(svg: "A")
        let bOffset = host.text.distance(from: host.text.startIndex, to: host.text.firstIndex(of: "b")!)
        host.record(host.engine.hover(location: bOffset, nowMs: host.now))
        host.advance(100)
        host.completeLatest(svg: "B")
        XCTAssertEqual(host.visible?.trigger, .hover)

        host.record(host.engine.hover(location: nil, nowMs: host.now))
        host.advance(200)
        let cOffset = host.text.distance(from: host.text.startIndex, to: host.text.firstIndex(of: "c")!)
        host.record(host.engine.hover(location: cOffset, nowMs: host.now))
        host.advance(300)
        XCTAssertEqual(host.renders.last?.source, "c")
        host.completeLatest(svg: "C")
        XCTAssertEqual(host.visible?.content, .fast(svg: "C"))
        XCTAssertEqual(host.visible?.trigger, .hover)
    }

    /// C13: persisted delay extremes — a stored negative is invalid
    /// (conservative 80), while stored 0/80/150 keep their exact meaning.
    func testPersistedDelayNormalizesInvalidValues() {
        for (stored, expected) in [(-1, 80), (Int.min, 80), (Int.max, 80), (0, 0), (80, 80), (150, 150), (999, 80)] {
            let s = EquationPreviewSettings(enabled: true, whileTyping: true, placement: "above", renderer: "fast", delayMilliseconds: stored)
            XCTAssertEqual(s.delayMilliseconds, UInt64(expected), "stored \(stored)")
        }
    }

    /// N5: Escape with a pending (not yet visible) math preview must
    /// consume the key and suppress that preview until the target changes.
    func testEscapeConsumesAPendingMathPreview() {
        var host = Host()
        host.open("$a‸$ and $b$")
        host.advance(100)
        host.completeLatest(svg: "A")
        XCTAssertNotNil(host.visible)
        var result = host.engine.escape(nowMs: host.now)
        host.record(result.commands)
        XCTAssertTrue(result.consumed)
        // Caret on $b$, fire still counting down: Escape dismisses the
        // pending preview before it ever renders.
        host.moveCaret((host.text as NSString).range(of: "b").location)
        let rendersBefore = host.renders.count
        result = host.engine.escape(nowMs: host.now)
        host.record(result.commands)
        XCTAssertTrue(result.consumed, "pending math preview must consume Escape")
        host.advance(200)
        XCTAssertEqual(host.renders.count, rendersBefore, "pending b never rendered")
        XCTAssertNil(host.visible)
        // A different equation is a new target: suppression ends there.
        host.moveCaret(1)
        host.advance(100)
        XCTAssertEqual(host.renders.last?.source, "a")
    }

    /// N5: a dormant target (stored without a live render, e.g. renderer
    /// not ready yet) must not be poisoned by Escape.
    func testEscapeIgnoresADormantTarget() {
        var host = Host()
        host.open("$a‸$", ready: false) // renderer never ready
        host.advance(200) // fire stored the target, nothing rendered
        XCTAssertNil(host.visible)
        let result = host.engine.escape(nowMs: host.now)
        host.record(result.commands)
        XCTAssertFalse(result.consumed, "dormant target is not dismissible work")
        host.record(host.engine.rendererReady(version: "test-renderer", nowMs: host.now))
        host.advance(100)
        XCTAssertEqual(host.renders.last?.source, "a")
    }

    /// N5: a pending fire on plain text is not an eligible preview —
    /// Escape passes through and nothing is suppressed.
    func testEscapeIgnoresAPendingPlainTextFire() {
        var host = Host()
        host.open("plain ‸text and $a$")
        host.advance(200) // first fire ran (hide), snapshot current
        // Re-schedule while still on plain text: fire deadline is pending.
        host.moveCaret((host.text as NSString).range(of: "x").location)
        let result = host.engine.escape(nowMs: host.now)
        host.record(result.commands)
        XCTAssertFalse(result.consumed, "pending plain-text fire must not consume Escape")
        host.moveCaret((host.text as NSString).range(of: "$a").location + 1)
        host.advance(100)
        XCTAssertEqual(host.renders.last?.source, "a")
    }

    /// N5: an in-flight render is consumable work — Escape cancels it and
    /// the late reply must not publish.
    func testEscapeCancelsInFlightRender() {
        var host = Host()
        host.open("$a‸$")
        host.advance(100)
        XCTAssertEqual(host.renders.count, 1) // issued, never completed
        let result = host.engine.escape(nowMs: host.now)
        host.record(result.commands)
        XCTAssertTrue(result.consumed)
        host.completeLatest(svg: "STALE")
        XCTAssertNil(host.visible)
        host.advance(500)
        XCTAssertNil(host.visible)
    }

    /// N5 r2: a second Escape inside the already-dismissed equation
    /// falls through — the pending fire was filtered as resolved-nothing,
    /// so the key is not consumed for a no-op.
    func testEscapeFallsThroughWhenPendingIsAlreadyDismissed() {
        var host = Host()
        host.open("$a‸$ and $b$")
        host.advance(100)
        host.completeLatest(svg: "A")
        var result = host.engine.escape(nowMs: host.now)
        host.record(result.commands)
        XCTAssertTrue(result.consumed)
        XCTAssertNil(host.visible)
        // Caret on $b$: first Escape dismisses the pending fire for b.
        let b = (host.text as NSString).range(of: "b").location
        host.moveCaret(b)
        result = host.engine.escape(nowMs: host.now)
        host.record(result.commands)
        XCTAssertTrue(result.consumed)
        // Move inside $b$ again (another pending fire on the SAME
        // dismissed region): Escape must now fall through.
        host.moveCaret(b + 1)
        result = host.engine.escape(nowMs: host.now)
        host.record(result.commands)
        XCTAssertFalse(result.consumed, "second Escape on the dismissed region is a no-op")
        host.advance(200)
        XCTAssertNil(host.visible)
    }

    /// N5 r2: after an edit the cached snapshot is stale, so a pending
    /// fire is unverifiable — Escape declines without touching engine
    /// state, and the fire then renders the updated text.
    func testEscapeDeclinesWhenTheSnapshotIsStale() {
        var host = Host()
        host.open("$a‸$ and tail")
        host.advance(100)
        host.completeLatest(svg: "A")
        // Retire the visible branch so this is purely the pending-fire path.
        var result = host.engine.escape(nowMs: host.now)
        host.record(result.commands)
        XCTAssertTrue(result.consumed)
        host.moveCaret((host.text as NSString).range(of: "tail").location)
        host.advance(200)
        // Edit: caret lands inside a DIFFERENT-offset equation ($ab$ at 2
        // — not the dismissed region at 0); snapshot revision no longer
        // matches, so eligibility is genuinely unverifiable.
        host.edit("x $ab‸$ and tail")
        let deadlineBefore = host.engine.nextDeadline
        XCTAssertNotNil(deadlineBefore, "edit schedules a fire")
        result = host.engine.escape(nowMs: host.now)
        host.record(result.commands)
        XCTAssertFalse(result.consumed, "stale snapshot: eligibility unknown, decline")
        XCTAssertTrue(result.commands.isEmpty, "declined Escape mutates nothing")
        XCTAssertEqual(host.engine.nextDeadline, deadlineBefore, "fire untouched")
        host.advance(200)
        XCTAssertEqual(host.renders.last?.source, "ab")
    }
}
