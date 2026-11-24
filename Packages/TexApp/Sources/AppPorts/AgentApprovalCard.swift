import Foundation

/// One button on an approval card (`approval_request.actions`).
public struct AgentApprovalAction: Equatable, Sendable {
    public var id: String
    public var label: String
    public var allows: Bool
    /// `primary`, `secondary`, `danger`, or empty.
    public var variant: String

    public init(id: String, label: String, allows: Bool, variant: String = "") {
        self.id = id
        self.label = label
        self.allows = allows
        self.variant = variant
    }

    /// Approve / Always allow / Deny, for a card the bridge sent no actions
    /// for, in the order and colours the ACP cards use.
    public static let legacy = [
        AgentApprovalAction(id: "approve", label: "", allows: true, variant: "primary"),
        AgentApprovalAction(id: "always_allow", label: "", allows: true),
        AgentApprovalAction(id: "deny", label: "", allows: false, variant: "danger"),
    ]

    /// `Localizable.strings` key for a known action id; nil means show `label`.
    public var labelKey: String? {
        switch id {
        case "approve", "allow_once": return "assistant.approve"
        case "deny", "reject_once": return "assistant.deny"
        case "always_allow", "allow_always": return "assistant.always_allow"
        case "reject_always": return "assistant.always_deny"
        case "approve_plan": return "assistant.plan.approve"
        case "approve_plan_accept_edits": return "assistant.plan.approve_accept_edits"
        case "keep_planning": return "assistant.plan.keep_planning"
        default: return nil
        }
    }
}

/// One AskUserQuestion item: choose an option (or several), or type "Other".
public struct AgentApprovalQuestion: Equatable, Sendable {
    public struct Option: Equatable, Sendable {
        public var label: String
        public var description: String
    }

    public var id: String
    public var header: String
    public var question: String
    public var options: [Option]
    public var multiSelect: Bool
    public var allowOther: Bool
}

/// What the user picked on the card on screen; `wireFields` is the
/// `approval_response` body without `type` and `id`.
public struct AgentApprovalAnswer: Equatable, Sendable {
    public enum Value: Equatable, Sendable {
        case one(String)
        case many([String])
    }

    /// `approve`, `deny`, or `always_allow`.
    public var decision: String
    public var actionID: String?
    /// Question id → the chosen label, labels, or "Other" text.
    public var answers: [String: Value]
    public var message: String?

    public init(decision: String, actionID: String? = nil, answers: [String: Value] = [:], message: String? = nil) {
        self.decision = decision
        self.actionID = actionID
        self.answers = answers
        self.message = message
    }

    /// A card button. The legacy three send only their decision, as before.
    public static func action(_ action: AgentApprovalAction, legacy: Bool, message: String? = nil) -> Self {
        if legacy { return Self(decision: action.id) }
        let decision: String
        switch action.id {
        case "approve", "deny", "always_allow": decision = action.id
        default: decision = action.allows ? "approve" : "deny"
        }
        return Self(decision: decision, actionID: action.id, message: message)
    }

    public var wireFields: [String: Any] {
        var fields: [String: Any] = ["decision": decision]
        if let actionID { fields["actionId"] = actionID }
        if !answers.isEmpty {
            fields["answers"] = answers.mapValues { value -> Any in
                switch value {
                case let .one(text): return text
                case let .many(labels): return labels
                }
            }
        }
        if let message = message?.trimmingCharacters(in: .whitespacesAndNewlines), !message.isEmpty {
            fields["message"] = message
        }
        return fields
    }
}

/// The bridge's permission mode (`get_state.mode`, `mode_changed.mode`).
public struct AgentPermissionMode: Equatable, Sendable {
    public struct Option: Equatable, Sendable, Identifiable {
        public var id: String
        public var label: String
    }

    public var id: String
    public var options: [Option]

    public init(id: String, options: [Option]) {
        self.id = id
        self.options = options
    }

    /// Nil without a mode or with no options (pi, Codex): the picker hides.
    public init?(_ value: Any?) {
        guard let object = value as? [String: Any], let id = object["id"] as? String,
              let rows = object["options"] as? [[String: Any]] else { return nil }
        let options = rows.compactMap { row -> Option? in
            guard let id = row["id"] as? String else { return nil }
            return Option(id: id, label: row["label"] as? String ?? id)
        }
        guard !options.isEmpty else { return nil }
        self.init(id: id, options: options)
    }

    /// `Localizable.strings` key for a known mode (Claude's or agy's id).
    /// Only Claude's default asks before edits (the bridge labels it so);
    /// agy's default edits project files without asking, so it reads "Default".
    public static func labelKey(id: String, label: String) -> String? {
        switch id {
        case "default": return label == "Ask before edits" ? "assistant.mode.default" : "assistant.mode.cli_default"
        case "acceptEdits", "accept-edits": return "assistant.mode.accept_edits"
        case "plan": return "assistant.mode.plan"
        default: return nil
        }
    }

    /// Edits apply without asking (accept-edits, or agy's default): the
    /// picker is tinted so it stays visible.
    public var autoAppliesEdits: Bool {
        let label = options.first { $0.id == id }?.label ?? id
        let key = Self.labelKey(id: id, label: label)
        return key == "assistant.mode.accept_edits" || key == "assistant.mode.cli_default"
    }
}

extension AgentApprovalItem {
    /// Decodes an `approval_request` event. Old bridges and pi send only
    /// id, kind, title and detail; everything else stays empty.
    public init(event object: [String: Any]) {
        func text(_ value: Any?) -> String {
            (value as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        }
        let id = text(object["id"])
        let kind = text(object["kind"])
        let title = text(object["title"])
        self.init(
            id: id,
            kind: kind.isEmpty ? "command" : kind,
            title: title.isEmpty ? "Approval" : title,
            detail: text(object["detail"])
        )
        toolDetail = AgentToolDetail(object["toolDetail"] as? [String: Any])
        actions = (object["actions"] as? [[String: Any]] ?? []).compactMap { row in
            let id = text(row["id"])
            let behavior = text(row["behavior"])
            guard !id.isEmpty, behavior == "allow" || behavior == "deny" else { return nil }
            let label = text(row["label"])
            return AgentApprovalAction(id: id, label: label.isEmpty ? id : label, allows: behavior == "allow",
                                       variant: text(row["variant"]))
        }
        questions = (object["questions"] as? [[String: Any]] ?? []).compactMap { row in
            let id = text(row["id"])
            let question = text(row["question"])
            guard !id.isEmpty, !question.isEmpty else { return nil }
            let options = (row["options"] as? [[String: Any]] ?? []).compactMap { option -> AgentApprovalQuestion.Option? in
                let label = text(option["label"])
                return label.isEmpty ? nil : .init(label: label, description: text(option["description"]))
            }
            return AgentApprovalQuestion(id: id, header: text(row["header"]), question: question, options: options,
                                         multiSelect: row["multiSelect"] as? Bool ?? false,
                                         allowOther: row["allowOther"] as? Bool ?? false)
        }
        if let plan = object["plan"] as? String, !plan.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            self.plan = String(plan.prefix(32_000))
        }
    }
}
