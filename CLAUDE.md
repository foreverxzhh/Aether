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
| pre-FIX_PATCH | 13 处隐性回退（MCP stderr、Delegate 占位、SSRF 字符串匹配、SecretString 缺失...） | 改完不自查 diff，编译器通过就交差 |

如果你再被抓到同类问题，花公子会停用你。
