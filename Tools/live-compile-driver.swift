// Line-protocol adapter; linked with the unmodified production scheduler.
import Foundation

@main struct Driver {
    static func main() {
        var scheduler = LiveCompileScheduler()
        var tokens: [UInt64: LiveRunToken] = [:]
        while let line = readLine() {
            let args = line.split(separator: " ")
            func number(_ index: Int) -> UInt64 { UInt64(args[index])! }
            var status = "-"
            let requests: [LiveRequest]
            switch args[0] {
            case "reset":
                scheduler = LiveCompileScheduler()
                tokens.removeAll()
                scheduler.setDelay(number(1))
                requests = scheduler.setEnabled(true)
            case "edit": requests = scheduler.noteEdit(nowMs: number(1))
            case "poll": requests = scheduler.poll(nowMs: number(1), composing: number(2) != 0)
            case "manual": requests = scheduler.requestManual()
            case "enable": requests = scheduler.setEnabled(number(1) != 0)
            case "invalidate": requests = scheduler.invalidate()
            case "finish":
                let result = scheduler.completed(token: tokens[number(1)]!,
                    nowMs: number(2), composing: number(3) != 0)
                switch result.status {
                case .current: status = "current"
                case .superseded: status = "superseded"
                case .stale: status = "stale"
                }
                requests = result.requests
            default: fatalError("unknown command: \(line)")
            }
            let encoded = requests.map { request in
                let kind: String
                let token: LiveRunToken
                switch request {
                case .startLive(let value, _): (kind, token) = ("live", value)
                case .startManual(let value): (kind, token) = ("manual", value)
                case .cancel(let value): (kind, token) = ("cancel", value)
                }
                tokens[token.id] = token
                return "\(kind):\(token.id)"
            }.joined(separator: ",")
            let deadline = scheduler.pendingDeadline.map { String($0) } ?? "-"
            let active = scheduler.active.map { String($0.id) } ?? "-"
            let accepts = scheduler.active.map { scheduler.acceptsActiveResult($0) } ?? false
            let response = "\(deadline)|\(active)|\(accepts ? 1 : 0)|\(status)|\(encoded)\n"
            FileHandle.standardOutput.write(Data(response.utf8))
        }
    }
}
