# FROZEN(2026-06-16)

iOS Swift SDK 已冻结。当前阶段聚焦 Android + Windows + Linux + macOS。

## 恢复步骤

1. 删除 `scripts/build-ios.sh` 中的 `exit 0` 行
2. 取消 `.github/workflows/ios.yml` 中的 `if: false`
3. 在 macOS 上运行 `bash scripts/build-ios.sh`
4. 生成新的 UniFFI Swift 绑定：`uniffi-bindgen swift agent-bindings/src/agent.udl`

## 目录内容

- `aether.swift` — UniFFI 自动生成的 Swift 绑定（当前可能已过时）
- `aetherFFI.h` — UniFFI 自动生成的 C 头文件
- `aetherFFI.modulemap` — Clang module map
- `Package.swift` — Swift Package Manager 配置
- `Sources/AetherSDK/Aether.swift` — 手写 Swift SDK 封装层
