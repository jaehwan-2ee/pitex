import AppPorts
import XCTest

final class AssistantCatalogTests: XCTestCase {
    private func model(_ id: String, _ provider: String, _ name: String) -> AssistantCatalog.Model {
        AssistantCatalog.Model(id: id, provider: provider, name: name, contextWindow: nil)
    }

    private func ready(_ backend: String, _ models: [AssistantCatalog.Model]) -> AssistantCatalog.Slot {
        AssistantCatalog.Slot(backend: backend, state: .ready(models))
    }

    private func titles(_ backend: String) -> String {
        switch backend {
        case "pi": "pi (provider sign-in)"
        case "claude": "Claude Code (your login)"
        case "apple-fm": "Apple Foundation Models (on this Mac)"
        default: backend
        }
    }

    private func facts(
        _ slots: [AssistantCatalog.Slot],
        active: String,
        current: AssistantCatalog.Model?,
        acceptsPicks: Bool = true
    ) -> AssistantCatalog.Facts {
        AssistantCatalog.Facts(
            slots: slots,
            activeBackend: active,
            current: current,
            generation: 3,
            acceptsPicks: acceptsPicks
        )
    }

    func testMenuMergesModelsFromEachReadyBackend() {
        let slots = [
            ready("pi", [model("sonnet", "anthropic", "Sonnet")]),
            ready("claude", [model("claude-sonnet", "claude", "Claude Sonnet")]),
        ]
        let menu = AssistantCatalog.menu(
            facts: facts(slots, active: "pi", current: model("sonnet", "anthropic", "Sonnet")),
            title: titles
        )
        XCTAssertEqual(menu.rows.map(\.label), [
            "pi (provider sign-in) · Sonnet (sonnet)",
            "Claude Code (your login) · Claude Sonnet (claude-sonnet)",
        ])
        XCTAssertEqual(menu.rows.map(\.section), [
            "pi (provider sign-in)",
            "Claude Code (your login)",
        ])
        XCTAssertEqual(menu.selection?.backend, "pi")
        XCTAssertTrue(menu.acceptsPicks)
    }

    func testMenuSuffixesTheRPCProviderWhenOneBackendListsSeveral() {
        let slots = [ready("pi", [
            model("sonnet", "anthropic", "Sonnet"),
            model("gpt", "openai", "gpt"),
        ])]
        let menu = AssistantCatalog.menu(facts: facts(slots, active: "pi", current: nil), title: titles)
        XCTAssertEqual(menu.rows.map(\.label), [
            "pi (provider sign-in) · Sonnet (sonnet) · anthropic",
            "pi (provider sign-in) · gpt · openai",
        ])
    }

    func testUnavailableBackendIsOmitted() {
        let slots = [
            ready("pi", [model("sonnet", "anthropic", "Sonnet")]),
            AssistantCatalog.Slot(backend: "claude", state: .unavailable),
            AssistantCatalog.Slot(backend: "apple-fm", state: .pending),
            ready("extra", []),
        ]
        let menu = AssistantCatalog.menu(facts: facts(slots, active: "pi", current: nil), title: titles)
        XCTAssertEqual(menu.rows.map(\.id.backend), ["pi"])
    }

    func testMacTargetsOmitHiddenCLIsAndAppleFMUntilItIsAvailable() {
        let hidden = AssistantCatalog.slots(backends: ["pi", "claude"]).map(\.backend)
        XCTAssertEqual(hidden, ["pi", "claude"])
        XCTAssertFalse(hidden.contains("codex"))
        XCTAssertFalse(hidden.contains("grok"))
        let mac = AssistantCatalog.slots(
            backends: ["pi", "claude", "apple-fm"],
            seeded: ["apple-fm": [AssistantCatalog.appleModel]]
        )
        let menu = AssistantCatalog.menu(facts: facts(mac, active: "pi", current: nil), title: titles)
        XCTAssertEqual(menu.rows.map(\.id.backend), ["apple-fm"])
        XCTAssertEqual(menu.rows.first?.id.modelID, "system")
        XCTAssertEqual(menu.rows.first?.id.provider, "apple")
    }

    func testSameBackendPickSetsOnlyTheModel() {
        let slots = [ready("pi", [
            model("sonnet", "anthropic", "Sonnet"),
            model("opus", "anthropic", "Opus"),
        ])]
        let current = model("sonnet", "anthropic", "Sonnet")
        let built = AssistantCatalog.menu(facts: facts(slots, active: "pi", current: current), title: titles)
        let session = AssistantCatalog.SessionSnapshot(isRunning: true, backend: "pi", current: current)
        let decided = AssistantCatalog.effect(
            intent: .row(built.rows[1].id),
            facts: facts(slots, active: "pi", current: current),
            shownGeneration: built.generation
        )
        let after = AssistantCatalog.sessionAfter(decided, session: session)
        XCTAssertEqual(decided, .setModel(model("opus", "anthropic", "Opus")))
        XCTAssertTrue(after.isRunning)
        XCTAssertEqual(after.backend, "pi")
        XCTAssertEqual(after.current?.id, "opus")
        XCTAssertEqual(
            AssistantCatalog.plan(
                intent: .row(built.rows[1].id),
                facts: facts(slots, active: "pi", current: current),
                shownGeneration: built.generation,
                session: session
            ),
            .setModel(model("opus", "anthropic", "Opus"))
        )
    }

    func testRepeatedPickOfTheCurrentModelDoesNothing() {
        let slots = [ready("pi", [model("sonnet", "anthropic", "Sonnet")])]
        let current = model("sonnet", "anthropic", "Sonnet")
        let built = AssistantCatalog.menu(facts: facts(slots, active: "pi", current: current), title: titles)
        let decided = AssistantCatalog.effect(
            intent: .row(built.selection!),
            facts: facts(slots, active: "pi", current: current),
            shownGeneration: 3
        )
        XCTAssertEqual(decided, .none)
    }

    func testCrossBackendPickSwitchesBackendAndModel() {
        let slots = [
            ready("pi", [model("sonnet", "anthropic", "Sonnet")]),
            ready("claude", [model("claude-sonnet", "claude", "Claude Sonnet")]),
        ]
        let current = model("sonnet", "anthropic", "Sonnet")
        let built = AssistantCatalog.menu(facts: facts(slots, active: "pi", current: current), title: titles)
        let session = AssistantCatalog.SessionSnapshot(isRunning: false, backend: "pi", current: current)
        let decided = AssistantCatalog.effect(
            intent: .row(built.rows[1].id),
            facts: facts(slots, active: "pi", current: current),
            shownGeneration: built.generation
        )
        let after = AssistantCatalog.sessionAfter(decided, session: session)
        XCTAssertEqual(decided, .switchBackend(backend: "claude", model: model("claude-sonnet", "claude", "Claude Sonnet")))
        XCTAssertEqual(after.backend, "claude")
        XCTAssertFalse(after.isRunning)
        XCTAssertEqual(after.current?.id, "claude-sonnet")
    }

    func testCrossBackendPickMidTurnClearsTheRunningTurn() {
        let slots = [
            ready("pi", [model("sonnet", "anthropic", "Sonnet")]),
            ready("claude", [model("claude-sonnet", "claude", "Claude Sonnet")]),
        ]
        let current = model("sonnet", "anthropic", "Sonnet")
        let built = AssistantCatalog.menu(facts: facts(slots, active: "pi", current: current), title: titles)
        let session = AssistantCatalog.SessionSnapshot(isRunning: true, backend: "pi", current: current)
        let decided = AssistantCatalog.effect(
            intent: .row(built.rows[1].id),
            facts: facts(slots, active: "pi", current: current),
            shownGeneration: 3
        )
        let after = AssistantCatalog.sessionAfter(decided, session: session)
        XCTAssertTrue(session.isRunning)
        XCTAssertFalse(after.isRunning)
        XCTAssertEqual(after.backend, "claude")
        XCTAssertEqual(
            AssistantCatalog.plan(
                intent: .row(built.rows[1].id),
                facts: facts(slots, active: "pi", current: current),
                shownGeneration: 3,
                session: session
            ),
            .switchBackend(backend: "claude", model: model("claude-sonnet", "claude", "Claude Sonnet"))
        )
    }

    func testCommitSelectsThePickedModelOnlyAfterTheNewBackendListsIt() {
        let wanted = model("claude-sonnet", "claude", "Claude Sonnet")
        let ticket = AssistantCatalog.SwitchTicket(backend: "claude", model: wanted)
        XCTAssertEqual(AssistantCatalog.commit(ticket: ticket, activeBackend: "pi", models: []), .hold)
        XCTAssertEqual(
            AssistantCatalog.commit(ticket: ticket, activeBackend: "claude", models: [model("other", "claude", "Other")]),
            .clear
        )
        XCTAssertEqual(AssistantCatalog.commit(ticket: ticket, activeBackend: "claude", models: [wanted]), .select(wanted))
        let settings = AssistantCatalog.SwitchTicket(backend: "claude", model: nil)
        XCTAssertEqual(AssistantCatalog.commit(ticket: settings, activeBackend: "claude", models: [wanted]), .clear)
    }

    func testStaleGenerationAndClosedPickerDoNotApplyARow() {
        let slots = [
            ready("pi", [model("sonnet", "anthropic", "Sonnet")]),
            ready("claude", [model("claude-sonnet", "claude", "Claude Sonnet")]),
        ]
        let built = AssistantCatalog.menu(facts: facts(slots, active: "pi", current: nil), title: titles)
        let stale = AssistantCatalog.effect(
            intent: .row(built.rows[1].id),
            facts: facts(slots, active: "pi", current: nil),
            shownGeneration: 2
        )
        XCTAssertEqual(stale, .none)
        let blocked = AssistantCatalog.effect(
            intent: .row(built.rows[1].id),
            facts: facts(slots, active: "pi", current: nil, acceptsPicks: false),
            shownGeneration: 3
        )
        XCTAssertEqual(blocked, .none)
        let settings = AssistantCatalog.effect(
            intent: .backend("claude"),
            facts: facts(slots, active: "pi", current: nil, acceptsPicks: false),
            shownGeneration: 3
        )
        XCTAssertEqual(settings, .switchBackend(backend: "claude", model: nil))
        let running = AssistantCatalog.SessionSnapshot(isRunning: true, backend: "pi", current: nil)
        XCTAssertFalse(AssistantCatalog.sessionAfter(settings, session: running).isRunning)
    }

    func testSettingsBackendForTheHiddenCLIsDoesNothing() {
        let slots = AssistantCatalog.slots(backends: ["pi", "claude"])
        let state = facts(slots, active: "pi", current: nil)
        for hidden in ["codex", "grok", "apple-fm", "pi"] {
            XCTAssertEqual(
                AssistantCatalog.effect(intent: .backend(hidden), facts: state, shownGeneration: 3),
                .none,
                hidden
            )
        }
    }

    func testMacCoordinatorAppliesASwitchByRestarting() throws {
        let coordinator = try source("Mac/Sources/Features/AgentCoordinator.swift")
        let apply = try functionBody(named: "func applyCatalog", in: coordinator)
        let setModel = apply.range(of: "case .setModel")!
        let switched = apply.range(of: "case .switchBackend")!
        let setPart = apply[setModel.lowerBound..<switched.lowerBound]
        let switchPart = apply[switched.lowerBound...]
        XCTAssertTrue(setPart.contains("selectModel"))
        XCTAssertFalse(setPart.contains("restart()"))
        XCTAssertTrue(switchPart.contains("restart()"))
        XCTAssertFalse(switchPart.contains("selectModel"))
        XCTAssertTrue(coordinator.contains("spawnAppleFM"))
        XCTAssertTrue(coordinator.contains("processEpoch.retire()"))
        XCTAssertTrue(coordinator.contains("isRunning = false"))
        let probe = try functionBody(named: "func startCatalogProbe", in: coordinator)
        XCTAssertFalse(probe.contains("processEpoch"))
        let panel = try source("Mac/Sources/Features/AgentPanel.swift")
        XCTAssertTrue(panel.contains("catalogMenu"))
        XCTAssertTrue(panel.contains("applyCatalog"))
        XCTAssertFalse(panel.contains("coordinator.restart()"))
        XCTAssertTrue(panel.contains("pitex.assistantModel"))
        for locale in ["en", "es", "ja", "ko", "ru", "vi", "zh-Hans"] {
            let table = try source("Mac/Resources/\(locale).lproj/Localizable.strings")
            XCTAssertTrue(table.contains("\"assistant.switched_backend\""), locale)
        }
    }

    private func functionBody(named name: String, in source: String) throws -> String {
        guard let start = source.range(of: name) else {
            XCTFail("missing \(name)")
            return ""
        }
        let rest = source[start.upperBound...]
        let markers = ["\n    func ", "\n    private func ", "\n    static func "]
        guard let next = markers.compactMap({ rest.range(of: $0) }).min(by: { $0.lowerBound < $1.lowerBound }) else {
            return String(source[start.lowerBound...])
        }
        return String(source[start.lowerBound..<next.lowerBound])
    }

    private func source(_ path: String) throws -> String {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return try String(contentsOf: url.appendingPathComponent(path), encoding: .utf8)
    }
}
