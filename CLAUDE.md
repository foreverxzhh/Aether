# Aether — Claude 协作规则

## 提交前强制自审

**每次 `git commit` 之前**，必须先跑自审脚本并通过：

```bash
bash scripts/self-audit.sh
```

不通过 = 不准提交。修复所有 ❌ 后再 commit。

## 自审脚本检查什么

| 阶段 | 检查内容 | 防止的问题 |
|------|---------|-----------|
| 0 | 编译 + 全部测试 | 最基本的不能坏 |
| 1 | 占位字符串（deferred / stub / TODO / unimplemented） | "假装修了" |
| 2 | 硬编码 Err 返回 | "表面实现，实际永远报错" |
| 3 | api_key 类型安全（SecretString） | API key 泄漏 |
| 4 | README 数字与代码一致 | 文档撒谎 |
| 5 | 关键实现真实性（MCP/Delegate/FTS5/SSRF/Terminal/Curator） | 核心功能回退 |
| 6 | 无效 feature gate 残留 | 死代码 |

## 编码铁律

1. **不写占位实现** — 如果功能没做完，函数体里不能是 `Err("deferred")` 或 `format!("[done]")` 这种占位字符串。真没做完就 `#[cfg(feature = "xxx")]` 整个函数 gate 掉，并在 FIX_PLAN 里登记
2. **不写假注释** — 不能写 "T-X.Y: 真实现见 future task" 然后不改代码
3. **改完立刻自审** — 不要等到最后一批 commit 才跑自审，每改一个文件就自查 diff
4. **数字要对齐** — 文档里写 "14 个工具"，代码里就要真有 14 个 impl Tool，差一个都不行
5. **中英文 README 同步改** — 改一处数字/状态，两个文件一起改
6. **新增检查规则** — 如果审计又发现新类型的问题，在 self-audit.sh 里新增检查，同时在本文档记录

## 自查流程

每次完成修改后，按这个顺序自查：

```
1. cargo build --workspace          # 必须 0 error
2. cargo test --workspace           # 必须全部通过
3. bash scripts/self-audit.sh       # 必须全部 ✅
4. git diff --stat                  # 确认改的文件都是预期的
5. git diff agent-core/src/         # 逐文件过一遍 diff，确认每行改动都有意义
6. git commit + push
```

第 5 步是最关键的——**不要用"编译过了、测试过了"骗自己**，用肉眼逐行看每处改动是否真的在做它声称要做的事。

## 审计历史教训

| 提交 | 被审计发现的问题 | 根因 |
|------|-----------------|------|
| pre-FIX_PATCH | 13 处隐性回退 | 改完不自查 diff |
| M1 (da68962) | 4 处真硬伤 (H1-H4) + 3 处习惯问题 (H5-H7) | acceptance 没当 checklist；RK 项列了不防 |

如果你再被抓到同类问题，花公子会停用你。

## Commit Message 规约（M2 起强制）

每个 feat/fix commit 必须包含 3 段：

### 实现段
列改动的关键 file:line（不要只写"实现 X"）：
- `agent-core/src/foo.rs:123` 添加 ...
- `tests/integration.rs:45` 新增 mock-based test ...

### 测试段（分类报）
- **真测试**（mock + 行为验证）: N 个
- **构造级测试**（仅 new + 断言非空）: M 个
- **不要只报 "+K 通过"，要报质量分布**

### 自审段
- 新增 self-audit grep: N 条（acceptance 里有要求时必须）
- self-audit.sh 全过: ✅ X + N 项

## 编码铁律（M1 hotfix 后追加）

7. **没做完不写解释性注释** — `// For now, X happens at Y level` 这种是大坑。改成 `unimplemented!("R-X.Y: do this")` 或 `#[cfg(feature = "foo")]`，**让编译器/运行时帮你抓**
8. **V1_ROADMAP RK 项先写检查再写功能** — 如果 roadmap 在风险登记里点名某事，先在 self-audit 加 grep，再做实现
9. **每个 PR 必改 CHANGELOG** — 哪怕一行 "Internal: refactored X"。breaking 改动必须显式标 `### Breaking`
10. **deprecated 立刻标，不留双份逻辑** — 引入新 API 同时把旧 API 标 `#[deprecated(since = "...", note = "use X")]`，下一个 minor 版本删
