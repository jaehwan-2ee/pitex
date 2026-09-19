import Foundation

/// One subscription rate-limit window as reported by the provider
/// (e.g. Anthropic's 5-hour/7-day buckets or Codex's primary/secondary
/// windows). `percent` is 0–100, matching what the provider's own UI shows.
struct UsageWindow: Hashable {
    let label: String
    let percent: Double
    let resetsAt: Date?
}

/// Fetches subscription rate-limit usage for OAuth providers from the same
/// endpoints the CLIs use, reading pi's `auth.json` for the access token.
/// API-key providers have no subscription windows — they keep the token/cost
/// display in `AgentPanel`.
@MainActor
final class SubscriptionUsageStore: ObservableObject {
    static let shared = SubscriptionUsageStore()

    /// Windows for the currently selected provider; nil when the provider is
    /// not OAuth-backed or the last fetch failed (panel falls back to
    /// token/cost display).
    @Published private(set) var windows: [UsageWindow]?
    @Published private(set) var providerID: String?

    private var task: Task<Void, Never>?
    private var lastFetch = Date.distantPast
    private let minimumInterval: TimeInterval = 60

    /// Refreshes usage for `provider`, at most once a minute. Called when the
    /// model changes and after each agent run.
    func refresh(provider: String?) {
        if provider != providerID {
            providerID = provider
            windows = nil
            lastFetch = .distantPast
        }
        guard let provider else { return }
        guard Date().timeIntervalSince(lastFetch) >= minimumInterval else { return }
        lastFetch = Date()
        task?.cancel()
        task = Task { [weak self] in
            let result = await Self.fetch(provider: provider)
            guard !Task.isCancelled else { return }
            self?.windows = result
        }
    }

    /// Reads the credential and queries the provider's usage endpoint.
    /// Returns nil for API-key credentials, unknown providers, missing
    /// tokens, or any network/parse failure — callers fall back gracefully.
    nonisolated private static func fetch(provider: String) async -> [UsageWindow]? {
        guard let credential = credential(for: provider),
              credential["type"] as? String == "oauth",
              let access = credential["access"] as? String, !access.isEmpty
        else { return nil }
        switch provider {
        case "anthropic":
            return await fetchAnthropic(access: access)
        case "openai-codex":
            return await fetchCodex(access: access, credential: credential)
        default:
            return nil
        }
    }

    private nonisolated static func credential(for provider: String) -> [String: Any]? {
        guard let data = try? Data(contentsOf: PiPaths.authFileURL),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }
        return object[provider] as? [String: Any]
    }

    // MARK: - Anthropic (Claude subscription)

    /// `GET https://api.anthropic.com/api/oauth/usage` — the same endpoint
    /// Claude Code reads. Buckets: `five_hour`, `seven_day`, and
    /// model-scoped `seven_day_*` (opus, sonnet, fable, …) depending on plan.
    private nonisolated static func fetchAnthropic(access: String) async -> [UsageWindow]? {
        var request = URLRequest(url: URL(string: "https://api.anthropic.com/api/oauth/usage")!)
        request.setValue("Bearer \(access)", forHTTPHeaderField: "Authorization")
        request.setValue("oauth-2025-04-20", forHTTPHeaderField: "anthropic-beta")
        request.timeoutInterval = 15
        guard let (data, response) = try? await URLSession.shared.data(for: request),
              (response as? HTTPURLResponse)?.statusCode == 200,
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }
        var windows: [UsageWindow] = []
        for (key, value) in object {
            guard let bucket = value as? [String: Any],
                  let utilization = bucket["utilization"] as? Double
            else { continue }
            let resetsAt = (bucket["resets_at"] as? String).flatMap(Self.parseISODate)
            switch key {
            case "five_hour":
                windows.append(UsageWindow(label: "5h", percent: utilization, resetsAt: resetsAt))
            case "seven_day":
                windows.append(UsageWindow(label: "7d", percent: utilization, resetsAt: resetsAt))
            case let k where k.hasPrefix("seven_day_"):
                // Model-scoped weekly buckets (seven_day_opus, seven_day_fable…).
                let name = k.replacingOccurrences(of: "seven_day_", with: "")
                    .replacingOccurrences(of: "_", with: " ")
                    .capitalized
                windows.append(UsageWindow(label: name, percent: utilization, resetsAt: resetsAt))
            default:
                continue
            }
        }
        // Stable order: 5h, 7d, then model-scoped buckets alphabetically.
        windows.sort { a, b in
            let rank = ["5h": 0, "7d": 1]
            return (rank[a.label] ?? 2, a.label) < (rank[b.label] ?? 2, b.label)
        }
        return windows.isEmpty ? nil : windows
    }

    // MARK: - OpenAI Codex (ChatGPT subscription)

    /// `GET https://chatgpt.com/backend-api/wham/usage` — the endpoint Codex
    /// reads. `primary_window`/`secondary_window` carry `used_percent` and
    /// `limit_window_seconds` (5h for Plus, 7d for Pro primary, etc.).
    private nonisolated static func fetchCodex(access: String, credential: [String: Any]) async -> [UsageWindow]? {
        var request = URLRequest(url: URL(string: "https://chatgpt.com/backend-api/wham/usage")!)
        request.setValue("Bearer \(access)", forHTTPHeaderField: "Authorization")
        if let accountID = codexAccountID(access: access, credential: credential) {
            request.setValue(accountID, forHTTPHeaderField: "ChatGPT-Account-Id")
        }
        request.timeoutInterval = 15
        guard let (data, response) = try? await URLSession.shared.data(for: request),
              (response as? HTTPURLResponse)?.statusCode == 200,
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let rateLimit = object["rate_limit"] as? [String: Any]
        else { return nil }
        var windows: [UsageWindow] = []
        for key in ["primary_window", "secondary_window"] {
            guard let window = rateLimit[key] as? [String: Any],
                  let used = window["used_percent"] as? Double
            else { continue }
            let seconds = window["limit_window_seconds"] as? Double
                ?? (window["limit_window_seconds"] as? Int).map(Double.init)
            let resetAt = (window["reset_at"] as? Double).map { Date(timeIntervalSince1970: $0) }
            windows.append(UsageWindow(label: windowLabel(seconds: seconds), percent: used, resetsAt: resetAt))
        }
        return windows.isEmpty ? nil : windows
    }

    /// The `ChatGPT-Account-Id` header value: a stored `accountId` field when
    /// present, otherwise the `chatgpt_account_id` claim inside the access
    /// token's JWT payload (same extraction pi performs).
    private nonisolated static func codexAccountID(access: String, credential: [String: Any]) -> String? {
        for key in ["accountId", "account_id"] {
            if let value = credential[key] as? String, !value.isEmpty { return value }
        }
        let parts = access.split(separator: ".")
        guard parts.count == 3,
              let payload = Data(base64URLEncoded: String(parts[1])),
              let claims = try? JSONSerialization.jsonObject(with: payload) as? [String: Any],
              let auth = claims["https://api.openai.com/auth"] as? [String: Any]
        else { return nil }
        return auth["chatgpt_account_id"] as? String
    }

    /// Labels a Codex window by its length: 5h/7d for the common buckets,
    /// `Nh`/`Nd` otherwise.
    private static func windowLabel(seconds: Double?) -> String {
        guard let seconds else { return "limit" }
        switch seconds {
        case ..<(6 * 3600): return "5h"
        case (6 * 86400)..<(8 * 86400): return "7d"
        default:
            let days = seconds / 86400
            if days >= 1 { return "\(Int(days.rounded()))d" }
            return "\(Int((seconds / 3600).rounded()))h"
        }
    }

    private nonisolated static func parseISODate(_ string: String) -> Date? {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter.date(from: string)
            ?? { let f = ISO8601DateFormatter(); return f.date(from: string) }()
    }
}

private extension Data {
    /// Base64url decoding for JWT payloads (no padding, `-`/`_` alphabet).
    init?(base64URLEncoded string: String) {
        var base64 = string
            .replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        base64 += String(repeating: "=", count: (4 - base64.count % 4) % 4)
        self.init(base64Encoded: base64)
    }
}
