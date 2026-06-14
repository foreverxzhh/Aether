package aether

import uniffi.aether.*

/**
 * Aether Agent — Android SDK
 *
 * 在 Android 应用中集成 AI Agent 能力。
 *
 * @property provider LLM 供应商 (openai / anthropic / deepseek / ollama)
 * @property model 模型名称
 * @property apiKey API Key（可选，也可用环境变量）
 *
 * @sample
 * ```kotlin
 * val agent = Aether(
 *     provider = "deepseek",
 *     model = "deepseek-v4-flash",
 *     apiKey = "sk-xxx"
 * )
 * agent.initModel()
 * val reply = agent.chat("你好")
 * println(reply)
 * ```
 */
class Aether(
    provider: String,
    model: String,
    apiKey: String? = null,
    baseUrl: String? = null,
    systemPrompt: String? = null,
) {
    private val inner: AetherAgent

    init {
        val config = SdkConfig(
            provider = provider,
            model = model,
            apiKey = apiKey,
            baseUrl = baseUrl,
            systemPrompt = systemPrompt,
        )
        inner = AetherAgent(config)
    }

    /**
     * 初始化 LLM 供应商。
     * 需在首次调用 [chat] 前调用一次。
     */
    fun initModel() {
        inner.initModel()
    }

    /**
     * 发送消息并获取回复。
     *
     * @param message 用户输入
     * @return Agent 的回复文本
     */
    fun chat(message: String): String {
        return inner.chat(message)
    }
}
