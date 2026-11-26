import Foundation

public enum LocalCloseChoice: Equatable, Sendable {
    case save
    case discard
    case cancel
}

public enum LocalCloseAction: Equatable, Sendable {
    case saveThenClose
    case closeWithoutSaving
    case stayOpen
}

public func localCloseAction(
    saveStates: [DocumentSaveState],
    choice: LocalCloseChoice?
) -> LocalCloseAction {
    let unsaved = saveStates.contains { $0 != .clean }
    guard unsaved else { return .closeWithoutSaving }
    switch choice {
    case .save:
        return .saveThenClose
    case .discard:
        return .closeWithoutSaving
    case .cancel, nil:
        return .stayOpen
    }
}

public enum ConflictCloseAction: Equatable, Sendable {
    case proceed
    case keepMine
    case useExternal
    case stayOpen
}

/// A conflicted file refuses every save, so local and SSH close both settle it first.
/// `.save` keeps the editor's text, `.discard` adopts the external file.
public func conflictCloseAction(
    saveStates: [DocumentSaveState],
    choice: LocalCloseChoice?
) -> ConflictCloseAction {
    guard saveStates.contains(.conflicted) else { return .proceed }
    switch choice {
    case .save:
        return .keepMine
    case .discard:
        return .useExternal
    case .cancel, nil:
        return .stayOpen
    }
}
