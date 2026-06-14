import Foundation
import AetherFFI

/// Aether Agent — Swift SDK for iOS/macOS
///
/// ```swift
/// let agent = Aether(provider: "deepseek", model: "deepseek-v4-flash")
/// agent.apiKey = "sk-xxx"
/// try await agent.initialize()
/// let reply = try await agent.chat("你好")
/// ```
public class Aether {
    private var inner: AetherAgent?
    public var provider: String
    public var model: String
    public var apiKey: String?
    public var baseUrl: String?
    public var systemPrompt: String?

    public init(provider: String, model: String) {
        self.provider = provider
        self.model = model
    }

    public func initialize() async throws {
        let config = SdkConfig(
            provider: provider,
            model: model,
            apiKey: apiKey,
            baseUrl: baseUrl,
            systemPrompt: systemPrompt
        )
        inner = AetherAgent(config: config)
        try inner?.initModel()
    }

    public func chat(_ message: String) async throws -> String {
        guard let agent = inner else {
            throw AetherError.runtimeError("Agent not initialized")
        }
        return try agent.chat(message: message)
    }
}

enum AetherError: Error {
    case runtimeError(String)
}
