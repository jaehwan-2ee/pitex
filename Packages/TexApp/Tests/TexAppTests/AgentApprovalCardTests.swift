import AppPorts
import XCTest

/// `approval_request` decode, the `approval_response` body, the permission
/// mode, and `approval_resolved` dropping a card — the same cases as the
/// Linux `agent.rs` tests.
final class AgentApprovalCardTests: XCTestCase {
    func testPlanCardKeepsActionsWithAKnownBehaviorAndThePlan() {
        let item = AgentApprovalItem(event: [
            "type": "approval_request", "id": "p", "kind": "plan", "title": "Review the plan",
            "detail": "## Plan", "plan": "## Plan\n1. Edit",
            "actions": [
                ["id": "approve_plan", "label": "Approve plan", "behavior": "allow", "variant": "primary"],
                ["id": "bogus", "label": "x", "behavior": "maybe"],
                ["id": "keep_planning", "label": "Keep planning", "behavior": "deny"],
            ],
        ])
        XCTAssertEqual(item.kind, "plan")
        XCTAssertEqual(item.plan, "## Plan\n1. Edit")
        XCTAssertEqual(item.actions.map(\.id), ["approve_plan", "keep_planning"])
        XCTAssertEqual(item.actions.map(\.allows), [true, false])
        XCTAssertEqual(item.actions.first?.variant, "primary")
        XCTAssertEqual(item.actions.first?.labelKey, "assistant.plan.approve")
    }

    func testQuestionCardDropsBlankOptionsAndQuestionsWithoutText() {
        let item = AgentApprovalItem(event: [
            "id": "q", "kind": "question", "title": "Q",
            "questions": [
                ["id": "0", "header": "Engine", "question": "Which engine?", "multiSelect": false, "allowOther": true,
                 "options": [["label": "pdfLaTeX", "description": "Fast"], ["label": ""], ["label": "LuaLaTeX"]]],
                ["id": "1", "header": "", "question": ""],
            ],
        ])
        XCTAssertEqual(item.questions.count, 1)
        let question = item.questions[0]
        XCTAssertEqual(question.options.map(\.label), ["pdfLaTeX", "LuaLaTeX"])
        XCTAssertEqual(question.options.map(\.description), ["Fast", ""])
        XCTAssertTrue(question.allowOther)
        XCTAssertFalse(question.multiSelect)
    }

    func testFileCardCarriesTheDiffAndOldBridgesKeepTheFourFields() {
        let edit = AgentApprovalItem(event: [
            "id": "e", "kind": "file", "title": "Edit files", "detail": "a.tex",
            "toolDetail": ["kind": "edit", "path": "a.tex", "diff": "@@ -1 +1 @@\n-a\n+b"],
        ])
        XCTAssertEqual(edit.toolDetail?.kind, .edit)
        XCTAssertEqual(edit.toolDetail?.diff, "@@ -1 +1 @@\n-a\n+b")
        XCTAssertTrue(edit.actions.isEmpty)
        XCTAssertNil(edit.plan)
        let legacy = AgentApprovalItem(event: ["id": "1"])
        XCTAssertEqual(legacy, AgentApprovalItem(id: "1", kind: "command", title: "Approval", detail: ""))
    }

    func testAnswersCarryTheActionTheAnswersAndTrimmedFeedback() {
        let keep = AgentApprovalAction(id: "keep_planning", label: "Keep planning", allows: false)
        let wire = AgentApprovalAnswer.action(keep, legacy: false, message: "  Also the abstract. ").wireFields
        XCTAssertEqual(wire["decision"] as? String, "deny")
        XCTAssertEqual(wire["actionId"] as? String, "keep_planning")
        XCTAssertEqual(wire["message"] as? String, "Also the abstract.")

        let questions = AgentApprovalAnswer(decision: "approve", answers: ["0": .one("XeLaTeX"), "1": .many(["Intro", "Method"])])
        let answers = questions.wireFields["answers"] as? [String: Any]
        XCTAssertEqual(answers?["0"] as? String, "XeLaTeX")
        XCTAssertEqual(answers?["1"] as? [String], ["Intro", "Method"])
        XCTAssertNil(questions.wireFields["actionId"])

        // The legacy three send only their decision, exactly as before.
        let legacy = AgentApprovalAnswer.action(AgentApprovalAction.legacy[1], legacy: true).wireFields
        XCTAssertEqual(legacy.count, 1)
        XCTAssertEqual(legacy["decision"] as? String, "always_allow")
        let once = AgentApprovalAction(id: "allow_once", label: "Allow", allows: true)
        XCTAssertEqual(AgentApprovalAnswer.action(once, legacy: false).decision, "approve")
        XCTAssertEqual(once.labelKey, "assistant.approve")
    }

    func testPermissionModeDecodesAndLabelsClaudeAndAgyModes() {
        let mode = AgentPermissionMode([
            "id": "accept-edits",
            "options": [["id": "default", "label": "Default"], ["id": "accept-edits"], ["id": "plan", "label": "Plan"]],
        ])
        XCTAssertEqual(mode?.options.map(\.id), ["default", "accept-edits", "plan"])
        XCTAssertEqual(mode?.options[1].label, "accept-edits")
        XCTAssertEqual(mode?.autoAppliesEdits, true)
        var agyDefault = mode
        agyDefault?.id = "default"
        XCTAssertEqual(agyDefault?.autoAppliesEdits, true, "agy's default edits without asking")
        let claude = AgentPermissionMode(["id": "default", "options": [["id": "default", "label": "Ask before edits"]]])
        XCTAssertEqual(claude?.autoAppliesEdits, false)
        XCTAssertNil(AgentPermissionMode(["id": "x", "options": [] as [Any]]), "no options: no picker")
        XCTAssertNil(AgentPermissionMode(nil), "pi reports no mode")
        XCTAssertEqual(AgentPermissionMode.labelKey(id: "default", label: "Ask before edits"), "assistant.mode.default")
        XCTAssertEqual(AgentPermissionMode.labelKey(id: "default", label: "Default"), "assistant.mode.cli_default")
        XCTAssertEqual(AgentPermissionMode.labelKey(id: "acceptEdits", label: "Accept edits"), "assistant.mode.accept_edits")
        XCTAssertNil(AgentPermissionMode.labelKey(id: "auto", label: "Auto"))
    }

    func testApprovalResolvedDropsOnlyThatCard() {
        var queue = AgentApprovalQueue()
        queue.enqueue(AgentApprovalItem(id: "1", kind: "command", title: "Run", detail: "a"))
        queue.enqueue(AgentApprovalItem(id: "2", kind: "command", title: "Run", detail: "b"))
        queue.remove(id: "1")
        XCTAssertEqual(queue.head?.id, "2")
        queue.remove(id: "9")
        XCTAssertEqual(queue.count, 1)
        queue.remove(id: "2")
        XCTAssertNil(queue.head)
    }
}
