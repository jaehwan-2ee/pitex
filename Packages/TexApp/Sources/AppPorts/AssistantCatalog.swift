import Foundation

/// One assistant conversation, plus a menu of every backend that has reported models.
/// Choosing another backend asks the caller to shut that conversation down first.
public enum AssistantCatalog {
    public struct Model: Equatable, Sendable {
        public var id: String
        public var provider: String
        public var name: String
        public var contextWindow: Int?

        public init(id: String, provider: String, name: String, contextWindow: Int?) {
            self.id = id
            self.provider = provider
            self.name = name
            self.contextWindow = contextWindow
        }

        public var pickerTitle: String { name == id ? id : "\(name) (\(id))" }
    }

    public struct RowID: Hashable, Sendable {
        public var backend: String
        public var modelID: String
        public var provider: String

        public init(backend: String, modelID: String, provider: String) {
            self.backend = backend
            self.modelID = modelID
            self.provider = provider
        }
    }

    public enum SlotState: Equatable, Sendable {
        case pending
        case ready([Model])
        case unavailable
    }

    public struct Slot: Equatable, Sendable {
        public var backend: String
        public var state: SlotState

        public init(backend: String, state: SlotState) {
            self.backend = backend
            self.state = state
        }
    }

    public struct Facts: Equatable, Sendable {
        public var slots: [Slot]
        public var activeBackend: String
        public var current: Model?
        public var generation: UInt64
        public var acceptsPicks: Bool

        public init(slots: [Slot], activeBackend: String, current: Model?, generation: UInt64, acceptsPicks: Bool) {
            self.slots = slots
            self.activeBackend = activeBackend
            self.current = current
            self.generation = generation
            self.acceptsPicks = acceptsPicks
        }
    }

    public struct Row: Equatable, Identifiable, Sendable {
        public var id: RowID
        public var model: Model
        public var label: String
        public var section: String

        public init(id: RowID, model: Model, label: String, section: String) {
            self.id = id
            self.model = model
            self.label = label
            self.section = section
        }
    }

    public struct Menu: Equatable, Sendable {
        public var rows: [Row]
        public var selection: RowID?
        public var acceptsPicks: Bool
        public var generation: UInt64

        public init(rows: [Row], selection: RowID?, acceptsPicks: Bool, generation: UInt64) {
            self.rows = rows
            self.selection = selection
            self.acceptsPicks = acceptsPicks
            self.generation = generation
        }
    }

    public enum SelectionIntent: Equatable, Sendable {
        case row(RowID)
        case backend(String)
    }

    public enum Effect: Equatable, Sendable {
        case none
        case setModel(Model)
        case switchBackend(backend: String, model: Model?)
    }

    /// What the assistant coordinator should do. A switch always leaves the turn idle.
    public enum Plan: Equatable, Sendable {
        case setModel(Model)
        case switchBackend(backend: String, model: Model?)
    }

    public struct SessionSnapshot: Equatable, Sendable {
        public var isRunning: Bool
        public var backend: String
        public var current: Model?

        public init(isRunning: Bool, backend: String, current: Model?) {
            self.isRunning = isRunning
            self.backend = backend
            self.current = current
        }
    }

    public struct SwitchTicket: Equatable, Sendable {
        public var backend: String
        public var model: Model?

        public init(backend: String, model: Model?) {
            self.backend = backend
            self.model = model
        }
    }

    public enum Commit: Equatable, Sendable {
        case hold
        case clear
        case select(Model)
    }

    public static let appleModel = Model(
        id: AppleFMCatalog.modelID,
        provider: AppleFMCatalog.modelProvider,
        name: AppleFMCatalog.modelName,
        contextWindow: AppleFMCatalog.defaultContextTokens
    )

    /// `seeded` backends start ready. Every other listed backend waits for a report.
    /// A backend that is not listed is not offered.
    public static func slots(backends: [String], seeded: [String: [Model]] = [:]) -> [Slot] {
        backends.map { backend in
            if let models = seeded[backend], !models.isEmpty {
                return Slot(backend: backend, state: .ready(models))
            }
            return Slot(backend: backend, state: .pending)
        }
    }

    public static func note(_ slots: inout [Slot], backend: String, state: SlotState) {
        guard let index = slots.firstIndex(where: { $0.backend == backend }) else { return }
        slots[index].state = state
    }

    public static func menu(facts: Facts, title: (String) -> String) -> Menu {
        var rows: [Row] = []
        for slot in facts.slots {
            guard case .ready(let models) = slot.state, !models.isEmpty else { continue }
            let section = title(slot.backend)
            let disambiguate = Set(models.map(\.provider)).count > 1
            for model in models {
                var label = "\(section) · \(model.pickerTitle)"
                if disambiguate { label += " · \(model.provider)" }
                rows.append(Row(
                    id: RowID(backend: slot.backend, modelID: model.id, provider: model.provider),
                    model: model,
                    label: label,
                    section: section
                ))
            }
        }
        let selection = rows.first { row in
            guard let current = facts.current else { return false }
            return row.id.backend == facts.activeBackend
                && row.model.id == current.id
                && row.model.provider == current.provider
        }?.id
        return Menu(
            rows: rows,
            selection: selection,
            acceptsPicks: facts.acceptsPicks && !rows.isEmpty,
            generation: facts.generation
        )
    }

    public static func effect(intent: SelectionIntent, facts: Facts, shownGeneration: UInt64) -> Effect {
        guard shownGeneration == facts.generation else { return .none }
        switch intent {
        case .backend(let backend):
            guard facts.slots.contains(where: { $0.backend == backend }), facts.activeBackend != backend else {
                return .none
            }
            return .switchBackend(backend: backend, model: nil)
        case .row(let id):
            guard facts.acceptsPicks, let model = model(in: facts.slots, id: id) else { return .none }
            if id.backend == facts.activeBackend,
               let current = facts.current, current.id == model.id, current.provider == model.provider {
                return .none
            }
            if id.backend == facts.activeBackend {
                return .setModel(model)
            }
            return .switchBackend(backend: id.backend, model: model)
        }
    }

    public static func sessionAfter(_ effect: Effect, session: SessionSnapshot) -> SessionSnapshot {
        switch effect {
        case .none:
            return session
        case .setModel(let model):
            return SessionSnapshot(isRunning: session.isRunning, backend: session.backend, current: model)
        case .switchBackend(let backend, let model):
            return SessionSnapshot(isRunning: false, backend: backend, current: model)
        }
    }

    public static func plan(
        intent: SelectionIntent,
        facts: Facts,
        shownGeneration: UInt64,
        session: SessionSnapshot
    ) -> Plan? {
        let decided = effect(intent: intent, facts: facts, shownGeneration: shownGeneration)
        guard sessionAfter(decided, session: session) != session else { return nil }
        switch decided {
        case .none:
            return nil
        case .setModel(let model):
            return .setModel(model)
        case .switchBackend(let backend, let model):
            return .switchBackend(backend: backend, model: model)
        }
    }

    /// The picked model is sent only after the new session reports that same id and provider.
    public static func commit(ticket: SwitchTicket?, activeBackend: String, models: [Model]) -> Commit {
        guard let ticket, ticket.backend == activeBackend else { return .hold }
        guard let wanted = ticket.model else { return .clear }
        if let found = models.first(where: { $0.id == wanted.id && $0.provider == wanted.provider }) {
            return .select(found)
        }
        return .clear
    }

    private static func model(in slots: [Slot], id: RowID) -> Model? {
        guard let slot = slots.first(where: { $0.backend == id.backend }),
              case .ready(let models) = slot.state else { return nil }
        return models.first { $0.id == id.modelID && $0.provider == id.provider }
    }
}
