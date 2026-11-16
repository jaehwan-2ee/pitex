import Foundation

public protocol SecurityScopedResource: AnyObject, Sendable {
    func startAccessing() -> Bool
    func stopAccessing()
}

public final class SecurityScopeLedger: @unchecked Sendable {
    public static let shared = SecurityScopeLedger()

    private struct Slot {
        var resource: any SecurityScopedResource
        var holders: Int
    }

    private let lock = NSLock()
    private var byPath: [String: Slot] = [:]
    private var pathByLease: [UUID: String] = [:]

    public init() {}

    public func begin(
        path: String,
        makeResource: @Sendable () -> any SecurityScopedResource
    ) -> UUID? {
        if let existing = join(path) {
            return existing
        }
        let resource = makeResource()
        guard resource.startAccessing() else {
            return nil
        }
        return store(path, resource: resource)
    }

    public func end(_ id: UUID) {
        let resource: (any SecurityScopedResource)?
        lock.lock()
        resource = take(id)
        lock.unlock()
        resource?.stopAccessing()
    }

    private func join(_ path: String) -> UUID? {
        lock.lock()
        defer { lock.unlock() }
        guard var slot = byPath[path], slot.holders > 0 else { return nil }
        slot.holders += 1
        byPath[path] = slot
        let id = UUID()
        pathByLease[id] = path
        return id
    }

    private func store(_ path: String, resource: any SecurityScopedResource) -> UUID {
        lock.lock()
        let extra: (any SecurityScopedResource)?
        let id = UUID()
        if var slot = byPath[path], slot.holders > 0 {
            extra = resource
            slot.holders += 1
            byPath[path] = slot
        } else {
            extra = nil
            byPath[path] = Slot(resource: resource, holders: 1)
        }
        pathByLease[id] = path
        lock.unlock()
        extra?.stopAccessing()
        return id
    }

    private func take(_ id: UUID) -> (any SecurityScopedResource)? {
        guard let path = pathByLease.removeValue(forKey: id) else { return nil }
        guard var slot = byPath[path] else { return nil }
        slot.holders -= 1
        if slot.holders > 0 {
            byPath[path] = slot
            return nil
        }
        byPath[path] = nil
        return slot.resource
    }
}

public final class SecurityScopeSession: @unchecked Sendable {
    private let ledger: SecurityScopeLedger
    private let lock = NSLock()
    private var ids: [UUID] = []

    public init(ledger: SecurityScopeLedger) {
        self.ledger = ledger
    }

    public func track(_ id: UUID) {
        lock.lock()
        ids.append(id)
        lock.unlock()
    }

    public func owns(_ id: UUID) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return ids.contains(id)
    }

    public func relinquish(_ id: UUID) {
        lock.lock()
        let index = ids.firstIndex(of: id)
        if let index { ids.remove(at: index) }
        lock.unlock()
        if index != nil { ledger.end(id) }
    }

    deinit {
        lock.lock()
        let leftover = ids
        ids.removeAll()
        lock.unlock()
        for id in leftover {
            ledger.end(id)
        }
    }
}
