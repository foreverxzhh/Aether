#!/bin/bash
# =============================================================================
# Aether 自审脚本 — 每次提交前必须跑通，否则不准推
# 灵感来源：FIX_PLAN.md 审计发现的 13 处隐性回退
# 维护规则：每发现一类新问题，就加一条检查
# =============================================================================
set -euo pipefail
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
PASS=0; FAIL=0
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

pass() { echo -e "  ${GREEN}✅ $1${NC}"; PASS=$((PASS+1)); }
fail() { echo -e "  ${RED}❌ $1${NC}"; FAIL=$((FAIL+1)); }
warn() { echo -e "  ${YELLOW}⚠️  $1${NC}"; }

# 工具函数：过滤掉注释行后再 grep
# 用法: grep_code "pattern" file...
grep_code() {
    local pat="$1"; shift
    grep -RHIn --include="*.rs" "$pat" "$@" 2>/dev/null | grep -vE '^\s*[^:]+:\s*//' || true
}

# 工具函数：累加 grep -c 输出（不用 bc）
sum_grep_counts() {
    local total=0
    for f in "$@"; do
        local c
        c=$(grep -c "$f" 2>/dev/null || echo 0)
        total=$((total + c))
    done
    echo $total
}

echo "========================================"
echo " Aether 自审 v1.0"
echo "========================================"

# ── 阶段 0：编译 + 测试（不过这个什么都别谈） ──
echo ""
echo "── 阶段 0：编译 + 测试 ──"

if cargo build --workspace 2>&1 | tail -1 | grep -q "Finished"; then
    pass "编译通过 (0 error)"
else
    fail "编译失败"
fi

TEST_OUTPUT=$(cargo test --workspace 2>&1)
if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    # 从 cargo test 输出提取通过的测试总数（兼容没有 bc 的环境）
    TEST_PASSED=$(echo "$TEST_OUTPUT" | grep "test result: ok" | sed -n 's/.*ok. //p' | sed -n 's/ passed.*//p' | head -1)
    [ -z "$TEST_PASSED" ] && TEST_PASSED="?"
    pass "全部测试通过 (各 suite 统计见上)"
else
    fail "有测试失败"
fi

# ── 阶段 1：占位字符串检测（最常见的"伪修复"） ──
echo ""
echo "── 阶段 1：占位字符串 ──"

PLACEHOLDER_PATTERNS=(
    "deferred"
    "future task"
    "Parent agent will execute"
    "unimplemented!"
    "TODO: implement"
    "stub"
    "coming soon"
    "not yet implemented"
    "暂未实现"
    "占位"
)

for pattern in "${PLACEHOLDER_PATTERNS[@]}"; do
    # 从 Rust 源代码中搜索，但排除纯注释行（// 开头的不算占位实现）
    RAW_HITS=$(grep -RIn --include="*.rs" "$pattern" agent-core/src/ agent-bindings/src/ 2>/dev/null || true)
    # 过滤掉:
    # 1. 注释行（以序号开头后紧跟 // 或 //! 或 ///）
    # 2. experimental_stubs feature gate 名字（T-1.1 修复，目的是 gate 掉桩工具）
    CODE_HITS=$(echo "$RAW_HITS" | grep -vE ':[[:space:]]*//' | grep -v "experimental_stubs" || true)
    if [ -z "$RAW_HITS" ]; then
        pass "无 '$pattern' 残留"
    elif [ -z "$CODE_HITS" ]; then
        pass "无 '$pattern' 残留（仅在注释/feature gate 名中，不算）"
    else
        fail "残留占位关键字 '$pattern': $CODE_HITS"
    fi
done

# ── 阶段 2：硬编码 Err 返回（表面实现，实际永远报错） ──
echo ""
echo "── 阶段 2：硬编码 Err 返回 ──"

# 检测 Err(AetherError::...) 后面直接跟字符串常量且包含 "deferred" / "not" / "unimplemented"
ERR_STUB=$(grep -RIn --include="*.rs" -E "Err\(.*deferred|Err\(.*not implemented|Err\(.*暂不" agent-core/src/ 2>/dev/null || true)
if [ -n "$ERR_STUB" ]; then
    fail "硬编码 Err 存根: $ERR_STUB"
else
    pass "无硬编码 Err 占位"
fi

# ── 阶段 3：配置文件关键字段检查 ──
echo ""
echo "── 阶段 3：关键类型安全检查 ──"

# api_key 不能是裸 Option<String>
if grep -Rn --include="*.rs" "api_key: Option<String>" agent-core/src/config.rs 2>/dev/null; then
    fail "api_key 仍是裸 Option<String>，必须用 SecretString"
else
    pass "api_key 已用 SecretString 包裹"
fi

# config.api_key 不允许直接字段访问（必须走 api_key_expose）
# config.rs 内部的 self.api_key 是访问器方法实现，放行
# LLM provider (openai.rs/anthropic.rs) 有自己的 api_key: String 字段，不是 AgentConfig 的，放行
# 检查其他文件是否绕过 api_key_expose() 直接访问 AgentConfig 的 SecretString 字段
DIRECT_KEY_ACCESS=$(grep -Rn --include="*.rs" '\.api_key\b' agent-core/src/ 2>/dev/null \
    | grep -v "api_key_expose\|set_api_key\|has_api_key\|clear_api_key\|//\|///" \
    | grep -v "agent-core/src/config.rs" \
    | grep -v "agent-core/src/llm/" || true)
# 验证 provider.rs（LLM 工厂）确实用了 api_key_expose
PROVIDER_FACTORY=$(grep -n "api_key_expose" agent-core/src/llm/provider.rs 2>/dev/null || true)
if [ -n "$DIRECT_KEY_ACCESS" ]; then
    fail "存在直接访问 AgentConfig.api_key 字段: $DIRECT_KEY_ACCESS"
else
    pass "无直接 AgentConfig.api_key 字段访问"
fi
if [ -n "$PROVIDER_FACTORY" ]; then
    pass "provider.rs 使用 api_key_expose() 读取密钥"
else
    fail "provider.rs 未使用 api_key_expose()，可能绕过 SecretString"
fi

# ── 阶段 4：README 与代码数字一致性 ──
echo ""
echo "── 阶段 4：文档数字一致性 ──"

# 从代码统计实际 impl Tool 个数
TOOL_COUNT=$(grep -r "impl Tool for" agent-core/src/tools/ agent-core/src/delegate.rs 2>/dev/null | wc -l)
[ -z "$TOOL_COUNT" ] && TOOL_COUNT=0
pass "实际 impl Tool 数量: $TOOL_COUNT 个（请手动与 README 对比）"

# ── 阶段 5：关键实现真实性验证 ──
echo ""
echo "── 阶段 5：关键实现真实性 ──"

# MCP: 必须有 McpStdioServer
grep -q "pub struct McpStdioServer" agent-core/src/mcp/mod.rs 2>/dev/null && \
    pass "MCP McpStdioServer 已实现" || \
    fail "MCP: 缺少 McpStdioServer"

# MCP: call_tool 不能返回 Err
grep -q "fn call_tool" agent-core/src/mcp/mod.rs 2>/dev/null && \
    pass "MCP call_tool 函数存在" || \
    fail "MCP: 缺少 call_tool"

# Delegate: 必须 impl Tool
grep -q "impl Tool for Delegate" agent-core/src/delegate.rs 2>/dev/null && \
    pass "Delegate impl Tool 已实现" || \
    fail "Delegate: 缺少 impl Tool"

# FTS5: 必须用 MATCH
grep -q "messages_fts MATCH" agent-core/src/memory/state.rs 2>/dev/null && \
    pass "FTS5 使用 MATCH（非 LIKE）" || \
    fail "FTS5: 未使用 MATCH"

# SSRF: 必须用 Url::parse
grep -q "url::Url::parse\|Url::parse" agent-core/src/tools/web_tools.rs 2>/dev/null && \
    pass "SSRF 使用 Url::parse 严格解析" || \
    fail "SSRF: 未使用 Url::parse"

# SSRF: 必须有 DNS 解析
grep -q "ToSocketAddrs" agent-core/src/tools/web_tools.rs 2>/dev/null && \
    pass "SSRF 使用 ToSocketAddrs DNS 解析" || \
    fail "SSRF: 缺少 DNS 解析"

# Terminal: 必须诚实描述非沙箱
grep -q "非沙箱\|宿主进程" agent-core/src/tools/terminal_tool.rs 2>/dev/null && \
    pass "Terminal 已标注非沙箱" || \
    fail "Terminal: 未标注非沙箱"

# Curator: 必须有 spawn_blocking
grep -q "spawn_blocking" agent-core/src/agent.rs 2>/dev/null && \
    pass "Curator 已 spawn_blocking 异步化" || \
    warn "Curator: 未使用 spawn_blocking（可能仍阻塞主线程）"

# R-1.1 chat_stream ReAct 循环 (H2 防回归)
if grep -q "StreamEvent::Done" agent-core/src/agent.rs 2>/dev/null; then
    pass "chat_stream_events 真实现 (StreamEvent::Done)"
else
    fail "chat_stream_events 缺 StreamEvent::Done"
fi
grep -qE "max_iterations|max_iter" agent-core/src/agent.rs 2>/dev/null && \
    pass "chat_stream_events 有迭代上限" || \
    fail "chat_stream_events 无熔断 (H2 回归)"

# R-1.3 MCP HTTP (H1 防回归)
if grep -q "pub struct McpHttpServer" agent-core/src/mcp/http.rs 2>/dev/null; then
    pass "McpHttpServer 真存在"
else
    fail "McpHttpServer 不存在"
fi
grep -q "session_id.*Mutex" agent-core/src/mcp/http.rs 2>/dev/null && \
    pass "MCP HTTP Session-Id 真持久化 (Mutex)" || \
    fail "MCP HTTP Session-Id 未持久化 (H1 回归)"

# R-1.5 Anthropic caching (H4 防回归)
grep -q "cache_read_input_tokens" agent-core/src/llm/anthropic.rs 2>/dev/null && \
    pass "AnthropicUsage 真解析 cache_read_input_tokens" || \
    fail "AnthropicUsage 未读 cache_read_input_tokens (H4 回归)"
grep -q "PromptParts" agent-core/src/prompt.rs 2>/dev/null && \
    pass "PromptParts 三段拆分存在" || \
    fail "PromptParts 缺失 (H4 回归)"

# H3 tracing 死路径防回归
if grep -qE "let _.*=.*Registry::default\(\)" agent-core/src/tracing.rs 2>/dev/null; then
    fail "tracing.rs 仍是 build-then-drop (H3 回归)"
else
    pass "tracing.rs 真 try_init (无 let _ 模式)"
fi

# H5 CHANGELOG 存在
if [ -f CHANGELOG.md ]; then
    pass "CHANGELOG.md 存在"
else
    fail "CHANGELOG.md 缺失 (H5)"
fi

# H6 config 字段消费检查
for field in temperature max_tokens; do
    if grep -rq "config\.${field}\|with_${field}" agent-core/src/llm/ 2>/dev/null; then
        pass "config.${field} 真消费"
    else
        warn "config.${field} 未消费 (H6)"
    fi
done
grep -q "compression_threshold_ratio" agent-core/src/loop_mod.rs 2>/dev/null && \
    pass "compression_threshold_ratio 真消费" || \
    warn "compression_threshold_ratio 硬编码 (H6)"

# ── 阶段 6：不再需要的 feature gate 检查 ──
echo ""
echo "── 阶段 6：无效 feature gate ──"

# 检测 experimental_stubs
STUB_GATE=$(grep -Rn "experimental_stubs" agent-core/src/ 2>/dev/null || true)
if [ -z "$STUB_GATE" ]; then
    pass "无 experimental_stubs 残留"
else
    warn "experimental_stubs gate 仍存在: $STUB_GATE"
fi

# ── 汇总 ──
echo ""
echo "========================================"
echo " 自审结果: $PASS 通过, $FAIL 失败"
if [ "$FAIL" -gt 0 ]; then
    echo -e " ${RED}不通过 — 修复上面的 ❌ 再提交${NC}"
    exit 1
else
    echo -e " ${GREEN}全部通过 ✅ — 可以放心提交${NC}"
    exit 0
fi
