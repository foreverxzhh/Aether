# Aether Android AI Shell — 设计文档

> **版本**: v0.1
> **日期**: 2026-06-21
> **状态**: 草案，待 Opus Review
> **关联**: [V1_ROADMAP.md](V1_ROADMAP.md) · [requirements.md](requirements.md) · [implementation-plan.md](implementation-plan.md)

---

## 一、愿景

### 1.1 一句话

**让 Aether 从一个「桌面 Agent SDK」进化为「Android AI Shell SDK」—— 开发者集成一个 AAR，就能在自己 App 里拥有一个能看屏幕、操控应用、管理通知、读写系统设置的 AI Agent。**

### 1.2 使用场景

```
用户对手机说 / 输入：「帮我订明天去北京的高铁，走二等座」

Aether Agent：
  1. open_app("12306")                          → 打开铁路12306
  2. get_screen()                                → 获取界面树
  3. click_element("出发站")                     → 点击
  4. type_text("天津")                           → 输入
  5. click_element("到达站") → type_text("北京")
  6. ... (选择日期、二等座、筛选、下单)
  7. get_notifications()                         → 等出票通知
  8. speak("已帮你订好 G102 天津→北京南，8:30出发")
```

**这不是一个 App，这是手机的新交互层。**

### 1.3 与豆包手机 / Rabbit R1 的本质区别

| | 豆包手机 / Rabbit R1 | Aether AI Shell |
|---|---|---|
| 实现方式 | 定制 ROM / 专用硬件 | 一个 APK，装在任何 Android 11+ 手机上 |
| AI 模型 | 厂商绑定 | 任意模型（DeepSeek / OpenAI / Anthropic / Ollama） |
| 开发者友好 | ❌ 封闭 | ✅ 开源 SDK，任何人可二次开发 |
| 底层 | 封闭 | 基于 AOSP 标准 API，不需要 root |

---

## 二、现状分析

### 2.1 Aether 当前能力矩阵

| 能力 | 状态 | 位置 |
|------|:--:|------|
| ReAct Agent 循环 | ✅ 完成 | [agent.rs:190-242](../agent-core/src/agent.rs#L190) |
| 流式输出 (StreamEvent) | ✅ 完成 | [agent.rs:272-399](../agent-core/src/agent.rs#L272) |
| 多 LLM 供应商 | ✅ 完成 | agent-core/src/llm/ |
| L1-L4 记忆系统 | ✅ 完成 | agent-core/src/memory/ |
| 技能系统 (agentskills.io) | ✅ 完成 | agent-core/src/skills/ |
| 学习闭环 (Review + Curator) | ✅ 完成 | agent-core/src/memory/review.rs + curator.rs |
| MCP 协议 | 🟡 部分 | stdio ✅ / HTTP ✅ / Server ✅ / OAuth ❌ |
| 上下文压缩 | 🟡 部分 | 逻辑存在，token估算CJK友好 |
| Android SDK (Kotlin) | 🟡 部分 | [Aether.kt](../sdks/android/src/main/java/aether/Aether.kt) |
| Android .so (arm64) | ✅ 就绪 | sdks/android/src/main/jniLibs/arm64-v8a/ |
| Android Demo App | ✅ 基本可用 | [MainActivity.kt](../examples/android-demo/app/src/main/java/com/aether/demo/MainActivity.kt) |

### 2.2 关键缺口：工具层是桌面向的

Aether 当前 14 个工具全部面向桌面 OS：

| 工具 | 桌面场景 | Android 场景 |
|------|---------|-------------|
| ReadFile / WriteFile | `/home/user/` | `/data/data/` 沙盒内可复用 |
| Terminal / Docker / SSH | bash / docker | ❌ Android 无原生终端 |
| WebSearch / WebExtract | 通用 | ✅ 可复用 |
| Memory / Skills | 通用 | ✅ 可复用 |
| **无障碍操控** | ❌ 不存在 | 🔴 **核心需求** |
| **通知读写** | ❌ 不存在 | 🔴 **核心需求** |
| **应用生命周期** | ❌ 不存在 | 🔴 **核心需求** |
| **截图/屏幕感知** | ❌ 不存在 | 🟡 重要 |
| **系统设置** | ❌ 不存在 | 🟡 重要 |

**结论：引擎完好，差一个 Android 工具层。**

---

## 三、架构设计

### 3.1 总览

```
┌──────────────────────────────────────────────────────────┐
│                   AI Shell App (APK)                      │
│                                                          │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐  │
│  │ Chat UI     │  │ Floating     │  │ Voice Input    │  │
│  │ (对话界面)   │  │ Panel        │  │ (语音输入)      │  │
│  └──────┬──────┘  │ (悬浮快捷面板)│  └────────────────┘  │
│         │         └──────┬───────┘                       │
│         └────────┬───────┘                               │
│                  │                                        │
│  ┌───────────────┴────────────────────────────────────┐  │
│  │              Aether Android SDK (Kotlin)            │  │
│  │                                                    │  │
│  │  ┌──────────────────┐  ┌────────────────────────┐  │  │
│  │  │ Rust Agent Core  │  │  Android Tool Layer     │  │  │
│  │  │ (via JNI/UniFFI) │  │  (Kotlin 原生实现)       │  │  │
│  │  │                  │  │                        │  │  │
│  │  │ • ReAct 循环     │  │  • ScreenTool           │  │  │
│  │  │ • LLM 调用       │  │  • AppManagerTool       │  │  │
│  │  │ • 记忆/技能      │  │  • NotificationTool     │  │  │
│  │  │ • 桌面工具       │  │  • SystemSettingsTool   │  │  │
│  │  │ • ForeignTool    │◄─┤  • MediaControlTool     │  │  │
│  │  │   桥接层          │  │  • DeviceActionTool     │  │  │
│  │  └──────────────────┘  └────────────────────────┘  │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │           Android System Services                  │  │
│  │  AccessibilityService · NotificationListener       │  │
│  │  PackageManager · ActivityManager · AlarmManager   │  │
│  │  MediaProjection · AudioManager · PowerManager     │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

### 3.2 核心设计决策：ForeignTool 桥接

**问题**：Rust 层无法直接调用 Android AccessibilityService / NotificationListener 等系统 API。

**方案**：在 Rust Tool trait 之上新增 `ForeignTool` —— 一个工具代理，其 `call()` 委托给 Kotlin 侧的回调函数。

```rust
// agent-core/src/tools/foreign.rs (新增)

use std::pin::Pin;
use std::future::Future;

/// 外部工具回调 — 由宿主语言（Kotlin/Swift/C#）实现
pub type ForeignToolCallback = Box<
    dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync,
>;

/// 一个由宿主语言实现的工具
pub struct ForeignTool {
    name: String,
    description: String,
    parameters: serde_json::Value,
    toolset_name: String,
    callback: ForeignToolCallback,
}

impl ForeignTool {
    pub fn new(
        name: &str,
        toolset: &str,
        description: &str,
        parameters: serde_json::Value,
        callback: ForeignToolCallback,
    ) -> Self { /* ... */ }
}

#[async_trait]
impl Tool for ForeignTool {
    fn name(&self) -> &str { &self.name }
    fn toolset(&self) -> &str { &self.toolset_name }
    fn description(&self) -> &str { &self.description }
    fn parameters(&self) -> Value { self.parameters.clone() }

    async fn call(&self, args: Value) -> Result<String, AetherError> {
        match (self.callback)(args).await {
            Ok(result) => Ok(result),
            Err(e) => Err(AetherError::ToolExecutionError(e)),
        }
    }
}
```

**Kotlin 侧用法**：

```kotlin
val agent = Aether(
    provider = "deepseek",
    model = "deepseek-v4-flash",
    apiKey = "sk-xxx"
)
agent.initModel()

// 注册 Android 原生工具
agent.registerTool(
    name = "get_screen",
    toolset = "android",
    description = "获取当前屏幕上的所有 UI 控件及其属性",
    parameters = """
        {
            "type": "object",
            "properties": {
                "filter": {"type": "string", "description": "可选：按文本/ID过滤控件"}
            }
        }
    """,
    handler = { args ->
        accessibilityService?.rootInActiveWindow?.toJson(filter = args["filter"]) ?: "{}"
    }
)

agent.registerTool(
    name = "click_element",
    toolset = "android",
    description = "点击屏幕上的某个 UI 控件",
    parameters = """
        {
            "type": "object",
            "properties": {
                "resource_id": {"type": "string"},
                "text": {"type": "string"},
                "description": {"type": "string"}
            }
        }
    """,
    handler = { args ->
        accessibilityService?.performClick(
            resourceId = args["resource_id"],
            text = args["text"],
            description = args["description"]
        ) ?: "未找到控件"
    }
)

// ... 更多工具
```

### 3.3 为什么不在 Rust 层实现 Android 工具

| 方案 | Rust JNI 实现 | ForeignTool (Kotlin 回调) |
|------|:--:|:--:|
| 开发效率 | ❌ 每个工具都要写 JNI 胶水 | ✅ Kotlin 直接写 |
| 调试体验 | ❌ NDK 调试困难 | ✅ Android Studio 断点 |
| 利用 AF 经验 | ❌ Rust 不是你的主场 | ✅ Kotlin 是你本行 |
| 编译速度 | ❌ 每次改工具要重编 Rust | ✅ 改 Kotlin 即时生效 |
| 性能 | ✅ 零 JNI 开销（除首次） | 🟡 每次工具调用多一次 JNI upcall |
| 代码量 | 多 3-5x | 少 |

**结论**：工具执行频率远低于 LLM 调用，一次 JNI upcall 的性能开销可忽略。选 ForeignTool。

### 3.4 Rust 侧需要改什么

1. **`agent-core/src/tools/foreign.rs`** — 新增 `ForeignTool` 结构体 + `Tool` trait 实现
2. **`agent-bindings/src/lib.rs`** — 暴露 `aether_register_foreign_tool()` C API
3. **`AIAgent` 新增 `register_foreign_tool()` 方法** — 运行时注册
4. **`ToolRegistry` 新增 toolset "android"** — 工具分组

改动量估计：Rust 侧 ~150 行，Kotlin 绑定 ~80 行。**对现有代码零破坏。**

---

## 四、Android 工具目录

### 4.1 工具全集（按优先级）

#### 🔴 Phase 1 — 核心（让 Agent 能「看」和「摸」）

| # | 工具名 | 功能 | 所需系统权限 |
|---|--------|------|------------|
| 1 | `get_screen` | 获取当前界面的 UI 控件树（AccessibilityNodeInfo 序列化为 JSON） | AccessibilityService |
| 2 | `click_element` | 通过 resourceId / text / description 定位控件并点击 | AccessibilityService |
| 3 | `type_text` | 向聚焦的输入框输入文字 | AccessibilityService |
| 4 | `scroll_screen` | 滚动当前界面 | AccessibilityService |
| 5 | `press_back` | 按返回键 | AccessibilityService (GLOBAL_ACTION_BACK) |
| 6 | `press_home` | 按 Home 键 | AccessibilityService (GLOBAL_ACTION_HOME) |
| 7 | `open_app` | 通过包名/应用名启动应用 | PackageManager + Intent |
| 8 | `get_current_app` | 获取当前前台应用信息 | UsageStatsManager 或 AccessibilityService |

**Phase 1 工期**：5-7 天

#### 🟡 Phase 2 — 感知（让 Agent 有「耳朵」和「记忆」）

| # | 工具名 | 功能 | 所需系统权限 |
|---|--------|------|------------|
| 9 | `get_notifications` | 获取当前通知栏内容列表 | NotificationListenerService |
| 10 | `clear_notification` | 清除指定通知 | NotificationListenerService |
| 11 | `screenshot` | 截取当前屏幕 | MediaProjection（需用户授权一次） |
| 12 | `get_clipboard` | 读取剪贴板 | ClipboardManager |
| 13 | `set_clipboard` | 写入剪贴板 | ClipboardManager |
| 14 | `get_device_state` | 获取设备状态（电量/网络/WiFi/蓝牙/音量/亮度） | 各种系统服务 |

**Phase 2 工期**：4-5 天

#### 🟢 Phase 3 — 执行（让 Agent 能「动手」）

| # | 工具名 | 功能 | 所需系统权限 |
|---|--------|------|------------|
| 15 | `send_sms` | 发送短信 | SmsManager（需用户授权） |
| 16 | `make_call` | 拨打电话 | Intent CALL_PHONE（需用户授权） |
| 17 | `set_alarm` | 设置闹钟 | AlarmManager |
| 18 | `take_photo` | 拍照 | Camera（需用户授权） |
| 19 | `manage_app` | 安装/卸载/强制停止应用 | PackageManager + 系统权限 |
| 20 | `system_setting` | 读写系统设置（亮度/音量/铃声等） | Settings.System（需 WRITE_SETTINGS） |
| 21 | `file_operation` | 在用户可访问的存储中读写文件 | SAF 或 File API |

**Phase 3 工期**：5-7 天

#### 🔵 Phase 4 — 交互（让 Agent 有「嘴」和「耳朵」）

| # | 工具名 | 功能 | 所需系统权限 |
|---|--------|------|------------|
| 22 | `speak` | TTS 语音播报 | TextToSpeech |
| 23 | `listen` | 语音识别输入 | SpeechRecognizer（需用户授权） |
| 24 | `show_overlay` | 显示悬浮窗（Agent 状态/结果展示） | SYSTEM_ALERT_WINDOW |
| 25 | `vibrate` | 振动反馈 | Vibrator |

**Phase 4 工期**：3-4 天

### 4.2 不做的工具

| 工具 | 原因 |
|------|------|
| 底层文件系统操作 | Android 沙盒限制，root 场景再考虑 |
| 修改其他应用数据 | 安全红线 |
| 伪造 GPS 位置 | 法律风险 |
| 自动支付/转账 | 安全红线，需 HITL 确认 |

---

## 五、SDK API 设计（Kotlin 侧）

### 5.1 核心类设计

```kotlin
package aether

/**
 * Aether Android Agent — 支持 Android 系统工具的 AI Agent
 */
class AetherAgent(
    val config: AgentConfig,
) {
    // ── 生命周期 ──
    fun initModel()
    fun close()

    // ── 对话 ──
    suspend fun chat(message: String): String
    fun chatStream(message: String): Flow<StreamEvent>

    // ── 工具管理 ──
    fun registerTool(
        name: String,
        toolset: String = "android",
        description: String,
        parameters: String,  // JSON Schema
        handler: suspend (args: Map<String, Any?>) -> String,
    )
    fun unregisterTool(name: String)
    fun listTools(): List<ToolInfo>

    // ── 配置 ──
    fun updateApiKey(key: String)
    fun setSystemPrompt(prompt: String)
}
```

### 5.2 使用示例（App 开发者视角）

```kotlin
class MyAIShellApp : Application() {

    lateinit var agent: AetherAgent

    override fun onCreate() {
        super.onCreate()

        // 1. 创建 Agent
        agent = AetherAgent(
            AgentConfig(
                provider = "deepseek",
                model = "deepseek-v4-flash",
                apiKey = BuildConfig.DEEPSEEK_API_KEY,
            )
        )
        agent.initModel()

        // 2. 注入 Android 工具
        AndroidToolProvider(agent, this).registerAll()
    }
}

// 工具提供者 — 可选捆绑或按需注册
class AndroidToolProvider(
    private val agent: AetherAgent,
    private val context: Context,
) {
    private var accessibilityService: AccessibilityService? = null
    private var notificationListener: NotificationListenerService? = null

    fun bindAccessibility(service: AccessibilityService) {
        this.accessibilityService = service
    }

    fun registerAll() {
        registerScreenTools()
        registerAppTools()
        registerNotificationTools()
        registerDeviceTools()
    }

    private fun registerScreenTools() {
        agent.registerTool(
            name = "get_screen",
            toolset = "android",
            description = "获取当前屏幕的 UI 控件树，返回 JSON",
            parameters = """{"type":"object","properties":{"max_depth":{"type":"number","default":20}}}""",
        ) { args ->
            accessibilityService?.rootInActiveWindow?.toJson(
                maxDepth = (args["max_depth"] as? Number)?.toInt() ?: 20
            ) ?: """{"error":"AccessibilityService 未绑定"}"""
        }

        agent.registerTool(
            name = "click_element",
            toolset = "android",
            description = "点击屏幕上的 UI 元素",
            parameters = """{
                "type":"object",
                "properties": {
                    "text": {"type":"string"},
                    "resource_id": {"type":"string"},
                    "description": {"type":"string"}
                }
            }""",
        ) { args ->
            val node = accessibilityService?.findNode(
                text = args["text"] as? String,
                resourceId = args["resource_id"] as? String,
                description = args["description"] as? String,
            )
            if (node != null) {
                node.performAction(AccessibilityNodeInfo.ACTION_CLICK)
                """{"success":true,"clicked":"${node.className}"}"""
            } else {
                """{"success":false,"error":"未找到匹配控件"}"""
            }
        }

        // ... 更多工具
    }
}
```

---

## 六、与现有路线图的关系

### 6.1 不冲突，是延伸

[V1_ROADMAP.md](V1_ROADMAP.md) 的 M1-M4 聚焦于把 Aether 做成**通用跨平台 Agent SDK**。本文档描述的是在 v1.0 基础上的**垂直场景延伸**——Android AI Shell。

| V1_ROADMAP 做什么 | 本文档做什么 |
|---|---|
| 4 端 SDK 就绪 | Android 端的**工具层**补齐 |
| 通用工具（文件/终端/Web） | Android **系统工具**（无障碍/通知/应用管理） |
| CLI / C# / Swift 示例 | 完整的 **Android AI Shell 参考实现** |
| MCP Server / OAuth | ForeignTool 桥接机制 |

### 6.2 建议时序

```
V1_ROADMAP (M1-M4)             本文档 (Android AI Shell)
═══════════════════             ════════════════════════════
                               │
M1: P0 核心补完                │  Phase 0: ForeignTool 桥接
  (进行中)                     │  (Rust 150行 + Kotlin 80行)
                               │
M2: 4 端 binding               │  Phase 1: 8 个核心工具
  (Android AAR 可发布)          │  (get_screen → open_app)
                               │
M3: P2 功能补完                │  Phase 2: 6 个感知工具
                               │  (通知 + 截图 + 设备状态)
                               │
M4: release 收尾               │  Phase 3-4: 执行 + 交互
                               │  (SMS/电话/系统设置/TTS)
                               │
v1.0-alpha                     │  AI Shell Demo 完整可跑
```

---

## 七、风险与缓解

| # | 风险 | 概率 | 缓解 |
|---|------|:--:|------|
| 1 | AccessibilityService JSON 序列化性能（大界面树可能 10KB+） | 中 | 限制深度、懒加载、增量变化 |
| 2 | 不同厂商 ROM 对 AccessibilityService 的限制（后台杀死） | 高 | 前台服务保活 + 电池优化白名单引导 |
| 3 | LLM 理解屏幕树 JSON 的效率 | 中 | 精简字段、只传关键属性、可选截图辅助 |
| 4 | Google Play 政策（无障碍权限需审核） | 中 | 明确用途说明、侧载分发也接受 |
| 5 | UniFFI async callback 稳定性 | 低 | ForeignTool 在 Rust 侧是同步 trait，异步由 Kotlin 协程处理 |

---

## 八、下一步行动

### 立即可做（本周）

1. **实现 ForeignTool** — `agent-core/src/tools/foreign.rs` + Kotlin 绑定
2. **写一个测试** — Mock LLM 调用 `get_screen` + `click_element`，验证工具走通
3. **在 Android Demo 里注册 3 个工具** — get_screen / click_element / open_app

### 等 V1_ROADMAP M2 完成后

4. **补全全部 25 个 Android 工具**
5. **写 AI Shell 参考 App**（替换现有 android-demo）
6. **真机测试** — 在多台不同厂商设备上验证无障碍兼容性

---

## 附录 A — 变更影响分析

| 受影响文件 | 变更类型 | 变更量 |
|-----------|---------|:--:|
| `agent-core/src/tools/foreign.rs` | **新增** | ~100 行 |
| `agent-core/src/tools/mod.rs` | 修改（+1 mod） | +1 行 |
| `agent-bindings/src/lib.rs` | 修改（+register_foreign_tool FFI） | ~50 行 |
| `sdks/android/.../Aether.kt` | 修改（+registerTool API） | ~60 行 |
| `sdks/android/.../AetherAgent.kt` | **新增** | ~150 行 |
| `examples/android-demo/` | 修改（+工具注册示例） | ~100 行 |
| 现有 Rust 代码 | **零破坏** | 0 行 |

---

## 附录 B — 与 Hermes 的关系

Aether 从 Hermes 继承了 Agent 引擎架构，但 Android 工具层是 Aether 独有的——Hermes 跑在 Termux 里，没有系统级 Android API 接入。这正是 Aether 相比 Hermes 的核心差异化价值：

| | Hermes on Android | Aether on Android |
|---|---|---|
| 运行方式 | Termux + Python | 原生 APK |
| 无障碍操控 | ❌ | ✅ |
| 通知读写 | ❌ | ✅ |
| 应用管理 | ❌ | ✅ |
| 分发 | 不可能（用户要装 Termux + 配环境） | 装个 APK |

---

> **给 Opus Reviewer 的指引**：请重点关注 §3.2 (ForeignTool 桥接设计) 和 §4.1 (工具优先级) 两部分。核心审查问题：① ForeignTool 的 callback 签名是否足够通用？② 工具优先级排序是否合理？③ 架构有没有过度设计？
