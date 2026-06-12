# Hermes Agent 源码架构深度分析

> 基于 Hermes Agent v0.16.0 源码
> 花了大量时间读 AGENTS.md（1370行开发者指南）+ 核心源码

---

## 一、代码规模概览

```
Hermes Agent 总共 ~30,000 行 Python (不含测试)
测试：~17,000 个测试，~900 个测试文件

核心引擎（必须重写的部分）：
  run_agent.py              5,400行  AIAgent 类 + 所有入口逻辑
  agent/conversation_loop.py 4,245行  run_conversation() 主循环体
  agent/context_compressor.py 2,258行  上下文压缩
  agent/prompt_builder.py    1,621行  系统提示词组装
  agent/background_review.py   608行  学习闭环触发器
  agent/curator.py           1,835行  技能生命周期管理
  agent/memory_manager.py      857行  记忆提供者编排
  model_tools.py             1,229行  工具编排层

工具系统：
  tools/registry.py           ~600行  工具注册/发现/分发
  tools/file_tools.py        ~1,500行 文件操作
  tools/terminal_tool.py     ~1,000行 终端执行
  tools/web_tools.py           ~800行 Web搜索/抓取
  tools/mcp_tool.py            ~600行 MCP协议
  其他 80+ 工具文件           各几百行

会话/状态：
  hermes_state.py            4,777行  SQLite会话存储 + FTS5

CLI/网关（可以跳过）：
  cli.py                    13,933行  CLI交互界面
  gateway/run.py             ~9,000行 消息网关
```

---

## 二、核心架构：五层模型

Hermes 没有微内核，没有插件系统框架，它的架构非常直接：

```
┌──────────────────────────────────────────────┐
│  入口层：CLI / Gateway / TUI / ACP / Batch    │
│  (cli.py, gateway/, ui-tui/, acp_adapter/)   │
├──────────────────────────────────────────────┤
│  编排层：AIAgent (run_agent.py)              │
│  • run_conversation() — 核心循环             │
│  • 三种 API 模式调度                          │
│  • 迭代预算控制                               │
│  • 上下文压缩触发                             │
│  • 学习闭环触发                               │
├──────────────┬──────────────┬────────────────┤
│  模型层      │  工具层       │  记忆层         │
│  agent/      │  tools/      │  agent/        │
│  anthropic_  │  registry.py │  memory_       │
│  adapter.py  │  + 80+工具   │  manager.py    │
│  chat_       │  model_tools │  curator.py    │
│  completion_ │  .py         │  background_   │
│  helpers.py  │  toolsets.py │  review.py     │
├──────────────┴──────────────┴────────────────┤
│  状态层：hermes_state.py (SQLite + FTS5)      │
├──────────────────────────────────────────────┤
│  配置层：config.yaml + .env + profiles        │
└──────────────────────────────────────────────┘
```

核心设计理念（引自 AGENTS.md）：

> **"The core is a narrow waist; capability lives at the edges."**
> 每个添加到核心的工具都会在每次 API 调用中发送给模型，所以门槛很高。

---

## 三、AIAgent 核心引擎详解

### 3.1 主循环 `run_conversation()`

```
一个 turn（用户发送一条消息到 Agent 回复完成）：

1. 组装 system prompt（只做一次，整个会话不变）
2. 组装 messages（系统 + 记忆 + 上下文文件 + 技能 + 历史 + 当前用户消息）
3. while (未到最大迭代 && 预算未耗尽) || 优雅调用:
   a. 重置本轮重试计数器
   b. 根据 api_mode 构建 API 请求参数
   c. 调用 LLM (流式)
   d. 解析响应：
      - 有 tool_calls → 执行工具 → 追加结果 → 继续循环
      - 有文本 → 这就是 final_response → break
      - 空响应 → 错误恢复
      - 截断响应 → 截断恢复
   e. 检查是否需要压缩上下文
4. 如果超预算还没有 final_response → 发送无工具的消息让模型总结
5. 返回 {"final_response": ..., "messages": [...]}
```

**关键特征：完全同步**。主循环是同步的，工具调用在 ThreadPoolExecutor 里并行执行（最多8个并发）。

### 3.2 三种 API 模式

| api_mode | 协议 | 供应商示例 |
|----------|------|-----------|
| `chat_completions` | OpenAI Chat Completions API | OpenAI, DeepSeek, OpenRouter, Ollama |
| `anthropic_messages` | Anthropic Messages API | Anthropic, MiniMax |
| `codex_responses` | OpenAI Codex Responses API | GPT-5.x, Codex |

每种模式有不同的消息格式、工具格式、流式格式。模式在 `_build_api_kwargs()` 中根据 provider 自动检测。

### 3.3 上下文压缩机制

触发条件：预估 token 超过模型上下文窗口的一定比例。

```
压缩流程：
1. 用辅助 LLM 对旧轮次做摘要
2. 头部和尾部受 token 预算保护
3. 压缩是会话生命周期事件：关闭当前 SQLite 会话行，创建子会话
4. 摘要预算约为压缩内容的 20%
5. 子会话通过 parent_session_id 链接
```

压缩后会退还一次迭代预算给主循环。

### 3.4 系统提示词的三层组装

```
稳定层（会话期间不变）：
  • 身份定义（"你是 Hermes Agent..."）
  • 当前日期时间
  • 平台提示（CLI/Gateway/...）
  • 用户 Soul 文件内容

上下文层（每 turn 刷新）：
  • AGENTS.md / CLAUDE.md / .cursorrules（工作目录的）
  • MEMORY.md（核心记忆）
  • USER.md（用户画像）

易变层（按需）：
  • 技能索引（加载了哪些 skill）
  • 工具使用指南
  • 环境信息（cwd, platform, OS）
```

**关键约束：系统提示词在会话期间不能变**——否则 Anthropic 的 prompt cache 会失效，成本飙升。

### 3.5 迭代预算控制

```python
IterationBudget:
  • 父 Agent 默认 90 次迭代
  • 子 Agent 默认 50 次
  • 工具全部是 execute_code → 退还一次迭代
  • 上下文压缩触发 → 退还一次迭代
  • 预算耗尽 → 发送无工具的消息让模型总结 → 这是"优雅调用"
```

### 3.6 错误恢复机制

| 错误类型 | 恢复策略 |
|---------|---------|
| 空响应 | 重试（带退避） |
| 截断响应 | 尝试修复（用 `finish_reason="length"`） |
| 无效工具调用 | 重试（告知模型错误） |
| JSON 解析失败 | 尝试修复（多种启发式） |
| Provider 报错 | 分类后重试或降级 |
| 上下文超限 | 触发压缩 |
| Circuit Breaker | 相同签名连续 N 次 → 返回错误强制模型换策略 |

---

## 四、工具系统

### 4.1 自注册模式

```
tools/registry.py          ← 单例 ToolRegistry (最底层，无依赖)
       ↓ import + register
tools/*.py                 ← 每个工具文件在模块级别调用 registry.register()
       ↓ 扫描 + import
model_tools.py             ← discover_builtin_tools() 扫描所有 tools/*.py
       ↓
run_agent.py, cli.py ...   ← 使用方
```

**注册方式**（每个工具文件）：

```python
from tools.registry import registry

registry.register(
    name="web_search",
    toolset="web",           # 归属于哪个工具集
    schema={...},            # OpenAI 格式的 JSON Schema
    handler=lambda args, **kw: web_search(...),
    check_fn=check_requirements,  # 可用性检查(是否有API Key等)
    requires_env=["SEARCH_API_KEY"],
    is_async=False,
    emoji="🔍",
)
```

### 4.2 工具集 (Toolset) 概念

`toolsets.py` 定义工具集。`_HERMES_CORE_TOOLS` 是默认集合（约50+个核心工具）。每个平台可以选不同的工具集：

```
_HERMES_CORE_TOOLS (所有平台默认有)
  ├── web: web_search, web_extract
  ├── file: read_file, write_file, patch, search_files
  ├── terminal: terminal, process
  ├── browser: browser_navigate, browser_snapshot, ...
  ├── skills: skills_list, skill_view, skill_manage
  ├── memory: memory
  ├── delegation: delegate_task
  ├── code_execution: execute_code
  └── ... 更多

平台特定工具集：
  messaging: send_message
  kanban: kanban_show, kanban_complete, ...
  homeassistant: ha_list_entities, ...
```

工具可用性由 `check_fn` 控制——如果 `check_fn()` 返回 False，工具不会出现在 schema 中。

### 4.3 工具执行

```python
# model_tools.py 中的 handle_function_call()
def handle_function_call(name, args, task_id, user_task):
    # 1. 某些工具被 run_agent.py 拦截（todo, memory）
    # 2. 通过 registry.dispatch() 分发
    # 3. 异步工具通过 event loop 桥接
    # 4. 所有错误都捕获并返回 JSON error
```

---

## 五、记忆系统（四层 L1-L4）

### 5.1 架构

```
MemoryManager (agent/memory_manager.py)
  ├── 内置核心记忆（MEMORY.md + USER.md）
  │     • 文件系统存储
  │     • 每次会话自动加载到 system prompt
  │
  └── 可插拔的 MemoryProvider（plugins/memory/*/）
        • MemoryProvider ABC（agent/memory_provider.py）
        • 8 个内置提供者: honcho, mem0, supermemory, byterover, ...
        • 每次 turn: sync_turn() → 写入
        • 每次 turn: prefetch() → 检索相关上下文
        • 只允许激活 1 个外部提供者

L1 核心记忆: MEMORY.md (环境事实、规范、偏好)
L2 用户画像: USER.md (技术栈、沟通风格)
L3 技能记忆: skills/*.md (可复用操作流程)
L4 长期存储: SQLite + FTS5 (完整对话历史)
```

### 5.2 会话存储 (hermes_state.py)

```
SessionDB (单例, SQLite + WAL 模式)
  ├── sessions 表：id, source, model, provider, config, ...
  ├── messages 表：session_id, role, content, tool_calls, ...
  ├── FTS5 虚拟表：全文搜索所有会话消息
  ├── 会话拆分：压缩时创建子会话（parent_session_id 链）
  └── 会话恢复：从任何 checkpoint 恢复
```

---

## 六、技能系统

### 6.1 技能格式 (agentskills.io 标准)

```markdown
---
name: my-skill
description: Clear one-line description of what this does.
version: 1.0.0
author: Author Name (@github)
license: MIT
platforms: [linux, macos]
metadata:
  hermes:
    tags: [tag1, tag2]
    category: devops
    config:
      MY_API_KEY: "API key for X service"
---

# My Skill

## When to Use
...

## How to Run
...

## Procedure
...

## Pitfalls
...
```

### 6.2 技能生命周期

```
创建 → 活跃 → 陈旧(30天不用) → 归档(90天) → [可恢复]
  ↑                ↓
  └─── patch 更新 ←┘
  
Curator (agent/curator.py) 管理自动流转：
  • 只处理 agent 创建的技能（created_by: "agent"）
  • Pin 的技能免于任何自动流转
  • 从不删除，最多归档
  • 归档前自动备份（tar.gz）
```

### 6.3 技能仓库

```
skills/               ← 内置技能（随仓库发布）
optional-skills/      ← 可选技能（需显式安装）
~/.hermes/skills/     ← 用户/Agent 生成的技能
~/.hermes/skills/.archive/  ← 归档技能（可恢复）
```

---

## 七、学习闭环

这是 Hermes 最核心的创新。分两个层次：

### 7.1 Turn 级：Background Review

```
每个 turn 结束后（每轮对话）
   ↓
spawn_background_review()（daemon 线程）
   ↓
fork 一个子 AIAgent（共享父 Agent 的配置和 session）
   ↓
子 Agent 用受限工具集（只有 memory + skill 工具）
   ↓
问自己两个问题：
  1. 用户有没有透露值得记住的信息？→ 写入 memory
  2. 有没有值得记录的操作流程/技巧？→ 创建/更新 skill
   ↓
fork Agent 结束。不触碰主 Agent 的 prompt cache。
```

### 7.2 会话级：Curator

```
定期检查（默认每 7 天，需要 Agent 空闲 2 小时）
   ↓
maybe_run_curator()
   ↓
fork 一个子 AIAgent（用 auxiliary 模型配置）
   ↓
审查所有 agent 创建的技能：
  • 哪些陈旧了（30天未用）→ 标记 stale
  • 哪些该归档了（90天）→ 移入 .archive/
  • 有重复的需要合并
  • 有过时的需要 patch
   ↓
生成 curator report
```

---

## 八、MCP 协议实现

### 8.1 架构

```
MCP Client（tools/mcp_tool.py）
  ├── stdio 传输：子进程 stdin/stdout JSON-RPC
  ├── HTTP/SSE 传输：HTTP POST + SSE 流
  ├── OAuth 管理：tools/mcp_oauth.py + mcp_oauth_manager.py
  └── 动态发现：
      • 启动时列出工具
      • 运行时响应 notifications/tools/list_changed
      • 每个 MCP 服务器映射为一个 toolset（mcp-<server_name>）

工具调用流程：
  1. 模型调用名为 "mcp__<server>__<tool>" 的工具
  2. model_tools.py 解析 server 和 tool 名
  3. 通过 JSON-RPC 调用 MCP 服务器的 tools/call
  4. 返回结果给模型
```

---

## 九、多 Profile 支持

```
每个 Profile 有独立的：
  ~/.hermes/profiles/<name>/
    ├── config.yaml
    ├── .env
    ├── sessions.db (SQLite)
    ├── skills/
    ├── memory/ (MEMORY.md, USER.md)
    └── logs/

Profile 之间完全隔离，通过 HERMES_HOME 环境变量实现。
```

---

## 十、配置体系

### 10.1 配置优先级

```
命令行参数 > config.yaml > DEFAULT_CONFIG > 硬编码默认值
```

### 10.2 配置分段

```
model:     模型选择、温度、max_tokens
agent:     Agent 行为参数
terminal:  终端后端 + 工作目录
compression: 上下文压缩参数
display:   UI 主题、皮肤
memory:    记忆提供者配置
delegation: 子 Agent 委托参数
curator:   技能策展人配置
skills:    技能相关配置
gateway:   消息网关配置
cron:      定时任务配置
profiles:  Profile 管理
plugins:   插件配置
```

---

## 十一、依赖链（核心模块间的导入关系）

```
hermes_constants.py          ← 最底层（路径工具函数）
    ↓
tools/registry.py            ← 工具注册表（无依赖）
    ↓
tools/*.py                   ← 各工具（只依赖 registry）
    ↓
model_tools.py               ← 工具编排（触发所有工具导入）
    ↓
agent/*.py                   ← Agent 内部模块（provider/记忆/压缩等）
    ↓
run_agent.py                 ← AIAgent 核心类
    ↓
cli.py / gateway/run.py      ← 入口
```

这个依赖链很简单：**单向、无环、逐层依赖**。

---

## 十二、关键文件速查表

| 文件 | 行数 | 核心职责 |
|------|------|---------|
| `run_agent.py` | 5,400 | AIAgent 类：构造函数、chat()、run_conversation() |
| `agent/conversation_loop.py` | 4,245 | run_conversation() 的循环体（从 run_agent.py 抽出） |
| `agent/context_compressor.py` | 2,258 | 上下文压缩（摘要+截断+会话拆分） |
| `agent/prompt_builder.py` | 1,621 | 系统提示词组装（三层结构） |
| `agent/background_review.py` | 608 | 学习闭环：每 turn 后异步审查 |
| `agent/curator.py` | 1,835 | 技能策展人：定期审查技能生命周期 |
| `agent/memory_manager.py` | 857 | 记忆提供者编排（L1-L4） |
| `agent/context_engine.py` | ? | 上下文注入引擎 |
| `agent/error_classifier.py` | ? | API 错误分类与恢复策略 |
| `agent/iteration_budget.py` | ~200 | 线程安全的迭代预算 |
| `agent/prompt_caching.py` | ~300 | Anthropic prompt cache 管理 |
| `model_tools.py` | 1,229 | 工具编排：发现、definitions、dispatch |
| `tools/registry.py` | ~590 | 工具注册表（单例，核心抽象） |
| `toolsets.py` | ? | 工具集定义 |
| `hermes_state.py` | 4,777 | SQLite 会话存储 + FTS5 |
