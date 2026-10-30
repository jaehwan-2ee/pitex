import Foundation

/// Token for one agent subprocess launch.
///
/// The exit callback cannot carry the process. A weak reference is nil
/// once that process has been released, and a nil check then matches the
/// process that is running now. Each launch takes the next token. An exit
/// applies only when its token is still the current launch.
public struct AgentProcessEpoch: Equatable, Sendable {
    private var current: UInt64 = 0

    public init() {}

    /// The token for a launch that replaces whatever was running.
    public mutating func beginLaunch() -> UInt64 {
        current += 1
        return current
    }

    /// Drops the current launch so its exit can no longer match.
    public mutating func retire() {
        current += 1
    }

    /// True only for the token `beginLaunch` handed to the live process.
    /// Zero is never a launch, so a missing token cannot match.
    public func isCurrent(_ token: UInt64) -> Bool {
        current != 0 && token == current
    }
}
