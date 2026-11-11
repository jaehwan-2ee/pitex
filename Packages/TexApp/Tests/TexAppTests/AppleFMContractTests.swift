import AppPorts
import XCTest

final class AppleFMContractTests: XCTestCase {
    func testAvailabilityReasonsUseDistinctSentences() {
        let reasons: [AppleFMUnavailableReason] = [
            .deviceNotEligible, .appleIntelligenceNotEnabled, .modelNotReady, .unsupportedSystem, .unknown,
        ]
        let messages = reasons.map(\.message)
        XCTAssertEqual(Set(messages).count, reasons.count)
        XCTAssertTrue(messages[0].contains("can't run Apple Intelligence"))
        XCTAssertTrue(messages[1].contains("Turn on Apple Intelligence"))
        XCTAssertTrue(messages[2].contains("still preparing"))
        XCTAssertTrue(messages[3].contains("macOS 26"))
        XCTAssertEqual(AppleFMCatalog.defaultContextTokens, 4096)
        XCTAssertEqual(AppleFMHost.unavailableMessage.contains("Mac app"), true)
    }

    func testPromptFitDropsEditorContextAndKeepsTheQuestion() {
        let document = String(repeating: "x", count: 20_000)
        let message = "<editor-context>\nCurrent content:\n\(document)\n</editor-context>\n\nFix the title"
        let fitted = AppleFMPromptBudget.fit(message, contextTokens: 4096)
        guard case .trimmed(let text) = fitted else {
            return XCTFail("expected a trimmed prompt, got \(fitted)")
        }
        XCTAssertFalse(text.contains(document))
        XCTAssertTrue(text.contains("Editor context omitted"))
        XCTAssertTrue(text.contains("Fix the title"))
    }

    func testShortPromptIsUnchanged() {
        let fitted = AppleFMPromptBudget.fit("Fix the title", contextTokens: 4096)
        XCTAssertEqual(fitted, .ready("Fix the title"))
    }

    func testHugeQuestionKeepsASuffixInsideTheBudget() {
        let question = String(repeating: "y", count: 50_000)
        let fitted = AppleFMPromptBudget.fit(question, contextTokens: 100)
        guard case .trimmed(let text) = fitted else {
            return XCTFail("expected a trimmed prompt")
        }
        XCTAssertLessThanOrEqual(text.utf8.count, AppleFMPromptBudget.inputByteBudget(contextTokens: 100))
        XCTAssertTrue(text.hasSuffix("yyy"))
    }

    func testTinyWindowRejectsThePrompt() {
        let fitted = AppleFMPromptBudget.fit("Hello", contextTokens: 0)
        guard case .rejected = fitted else {
            return XCTFail("expected a rejection")
        }
    }

    func testSnapshotsBecomeTextDeltas() {
        var cursor = AppleFMStreamCursor()
        let first = cursor.consume(snapshot: "Hel")
        let second = cursor.consume(snapshot: "Hello")
        XCTAssertEqual(deltaText(in: first), "Hel")
        XCTAssertEqual(deltaText(in: second), "lo")
        XCTAssertEqual(first.first?.string("type"), "agent_start")
        let closed = cursor.close(stopReason: "stop", errorMessage: nil)
        XCTAssertEqual(closed.map { $0.string("type") }, ["message_end", "turn_end", "agent_end"])
        XCTAssertEqual(closed[0].object("message")?["stopReason"], .string("stop"))
    }

    func testADivergingSnapshotWaitsForTheFinalMessage() {
        var cursor = AppleFMStreamCursor()
        _ = cursor.consume(snapshot: "Hello")
        let diverged = cursor.consume(snapshot: "Reset")
        XCTAssertEqual(deltaText(in: diverged), nil)
        let closed = cursor.close(stopReason: "stop", errorMessage: nil)
        guard case .array(let items)? = closed[0].object("message")?["content"] else {
            return XCTFail("missing final content")
        }
        XCTAssertEqual(items.first?.string("text"), "Reset")
    }

    func testCancelClosesTheTurnWithAnErrorThePanelCanShow() {
        var cursor = AppleFMStreamCursor()
        _ = cursor.consume(snapshot: "Hel")
        let closed = cursor.close(stopReason: "error", errorMessage: "The turn was cancelled.")
        let end = closed.last
        XCTAssertEqual(end?.string("type"), "agent_end")
        let message = end?.array("messages")?.first
        XCTAssertEqual(message?.string("stopReason"), "error")
        XCTAssertEqual(message?.string("errorMessage"), "The turn was cancelled.")
    }

    func testReadDoesNotAskAndWritesAndCommandsDo() {
        XCTAssertEqual(AppleFMTools.permission(for: AppleFMTools.read), .allow)
        XCTAssertEqual(
            AppleFMTools.permission(for: AppleFMTools.write),
            .ask(kind: "file", title: "Write file")
        )
        XCTAssertEqual(
            AppleFMTools.permission(for: AppleFMTools.run),
            .ask(kind: "command", title: "Run command")
        )
        XCTAssertEqual(AppleFMTools.permission(for: "web_search"), .refuse)
        XCTAssertEqual(
            AppleFMTools.permission(for: AppleFMTools.write, rememberedKinds: ["file"]),
            .allow
        )
        XCTAssertEqual(AppleFMApprovalDecision(wire: "approve"), .approve)
        XCTAssertEqual(AppleFMApprovalDecision(wire: "always_allow"), .alwaysAllow)
        XCTAssertEqual(AppleFMApprovalDecision(wire: "nope"), .deny)
    }

    func testPathsStayInsideTheProject() {
        XCTAssertEqual(AppleFMPaths.relative("notes.tex"), "notes.tex")
        XCTAssertEqual(AppleFMPaths.relative("sections/intro.tex"), "sections/intro.tex")
        XCTAssertNil(AppleFMPaths.relative("../secrets"))
        XCTAssertNil(AppleFMPaths.relative("/etc/passwd"))
        XCTAssertNil(AppleFMPaths.relative("dir/../../outside"))
        XCTAssertNil(AppleFMPaths.relative(""))
    }

    func testCatalogRepliesMatchThePanelContract() {
        let state = AppleFMEvents.state(contextTokens: 4096, id: "req-1")
        XCTAssertEqual(state.string("command"), "get_state")
        XCTAssertEqual(state.string("id"), "req-1")
        XCTAssertEqual(state["success"], .bool(true))
        XCTAssertEqual(state.object("data")?["thinkingLevel"], .string("off"))
        XCTAssertEqual(state.object("data")?["model"]?.string("id"), "system")
        XCTAssertEqual(state.object("data")?["model"]?.string("provider"), "apple")
        XCTAssertEqual(state.object("data")?["model"]?["contextWindow"], .int(4096))

        let levels = AppleFMEvents.thinkingLevels(id: "req-1")
        XCTAssertEqual(levels.object("data")?["levels"], .array([.string("off")]))

        let rejected = AppleFMEvents.setThinkingLevel("high", id: nil)
        XCTAssertEqual(rejected["success"], .bool(false))

        let kept = AppleFMEvents.setModel(provider: "apple", modelID: "system", contextTokens: 4096, id: nil)
        XCTAssertEqual(kept["success"], .bool(true))
        let other = AppleFMEvents.setModel(provider: "openai", modelID: "gpt", contextTokens: 4096, id: nil)
        XCTAssertEqual(other["success"], .bool(false))
    }

    func testSwitchingProviderMidTurnRetiresTheAppleFMTurn() throws {
        var epoch = AgentProcessEpoch()
        let live = epoch.beginLaunch()
        var cursor = AppleFMStreamCursor()
        XCTAssertEqual(deltaText(in: cursor.consume(snapshot: "Hel")), "Hel")
        let openTurn = cursor

        epoch.retire()
        XCTAssertFalse(epoch.isCurrent(live))

        var lateTurn = openTurn
        let late = lateTurn.consume(snapshot: "Hello")
        XCTAssertEqual(deltaText(in: late), "lo")
        XCTAssertTrue(events(late, acceptedBy: epoch, token: live).isEmpty)

        var cancelTurn = openTurn
        let cancelled = cancelTurn.close(stopReason: "error", errorMessage: "The turn was cancelled.")
        XCTAssertEqual(cancelled.last?.string("type"), "agent_end")
        XCTAssertTrue(events(cancelled, acceptedBy: epoch, token: live).isEmpty)

        let replacement = epoch.beginLaunch()
        XCTAssertTrue(epoch.isCurrent(replacement))
        XCTAssertFalse(epoch.isCurrent(live))
        XCTAssertEqual(deltaText(in: events(late, acceptedBy: epoch, token: replacement)), "lo")

        let coordinator = try source("Mac/Sources/Features/AgentCoordinator.swift")
        XCTAssertTrue(coordinator.contains("guard let self, self.processEpoch.isCurrent(token) else { return }"))
        XCTAssertTrue(coordinator.contains("for await event in session.events"))
        XCTAssertTrue(coordinator.contains("connect(session)"))
        XCTAssertTrue(coordinator.contains("isRunning = false"))
        XCTAssertTrue(coordinator.contains("process?.terminate()"))
    }

    func testPickerSourcesGateAppleFMToTheMacApp() throws {
        let settings = try source("Mac/Sources/Features/SettingsView.swift")
        XCTAssertTrue(settings.contains("case appleFM = \"apple-fm\""))
        XCTAssertTrue(settings.contains("settings.ai.backend_apple_fm"))
        XCTAssertTrue(settings.contains("case .pi, .claude, .appleFM: true"))
        XCTAssertTrue(settings.contains("AppleFMRuntime.availability()"))
        XCTAssertTrue(settings.contains("ForEach(ShippedAgentProvider.selectableCases"))
        let coordinator = try source("Mac/Sources/Features/AgentCoordinator.swift")
        XCTAssertTrue(coordinator.contains("spawnAppleFM"))
        let rust = try source("Linux/crates/pitex-shell/src/settings.rs")
        XCTAssertTrue(rust.contains("OnDevice"))
        XCTAssertTrue(rust.contains("visible_agent_providers"))
        XCTAssertFalse(rust.contains("fn selectable_providers"))
        XCTAssertTrue(rust.contains(AppleFMHost.unavailableMessage))
        let panes = try source("Linux/crates/pitex-shell/src/panes.rs")
        XCTAssertTrue(panes.contains("visible_agent_providers"))
        XCTAssertFalse(panes.contains("selectable_providers"))
        XCTAssertFalse(panes.contains("DropDown::from_strings(&label_refs)") && panes.contains("AGENT_PROVIDER_CHOICES\n        .iter()\n        .map(|choice| tr(lang, choice.label_key))"))
    }

    private func events(
        _ events: [AppleFMJSON],
        acceptedBy epoch: AgentProcessEpoch,
        token: UInt64
    ) -> [AppleFMJSON] {
        epoch.isCurrent(token) ? events : []
    }

    private func deltaText(in events: [AppleFMJSON]) -> String? {
        events.compactMap { event -> String? in
            event.object("assistantMessageEvent")?["delta"]?.stringValue
        }.first
    }

    private func source(_ path: String) throws -> String {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return try String(contentsOf: url.appendingPathComponent(path), encoding: .utf8)
    }
}

private extension AppleFMJSON {
    var stringValue: String? {
        if case .string(let value) = self { return value }
        return nil
    }
}
