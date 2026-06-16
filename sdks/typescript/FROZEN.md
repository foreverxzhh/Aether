# FROZEN(2026-06-16)

Web/WASM TypeScript SDK 已冻结。当前阶段聚焦 Android + Windows + Linux + macOS。

## 恢复步骤

1. 取消 `Cargo.toml` workspace members 中 `"agent-wasm"` 的注释
2. 取消 `rust-toolchain.toml` targets 中 `wasm32-unknown-unknown` 的注释
3. 取消 `agent-bindings/Cargo.toml` 中 wasm 依赖和 feature 的注释
4. 取消 `agent-bindings/src/lib.rs` 中 `pub mod wasm` 的注释
5. 运行 `wasm-pack build --target web agent-wasm/`
6. 将生成的 pkg/ 内容复制到此目录

## 目录内容

- `src/index.ts` — TypeScript SDK 入口
- `src/wasm/` — WASM 二进制 + JS 胶水代码 + 类型声明
- `examples/index.html` — 浏览器 Demo 页面
