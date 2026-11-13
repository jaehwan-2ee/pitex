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

/// A local close may drop text only after the user says so.
/// Clean sessions close with no prompt. Dirty and conflicted sessions stay
/// open until the user picks Save or Don't Save.
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
