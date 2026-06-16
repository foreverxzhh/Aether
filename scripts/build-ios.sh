#!/bin/bash
# FROZEN(2026-06-16): iOS 构建已冻结。需要恢复时删除下面这行 exit 0 即可。
exit 0
# ============================================================
# 以下为原始构建逻辑，冻结保留
# 构建 Aether iOS SDK
# 前置: macOS + Xcode + rustup target add aarch64-apple-ios
set -e

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
IOS_DIR="$PROJECT_DIR/sdks/ios"

echo "🔨 构建 Aether iOS SDK"

# 编译 iOS 原生库
rustup target add aarch64-apple-ios 2>/dev/null || true
cargo build -p agent-bindings --release --target aarch64-apple-ios --no-default-features

# 生成 Swift 绑定
uniffi-bindgen generate "$PROJECT_DIR/agent-bindings/src/agent.udl" \
    --language swift --out-dir "$IOS_DIR"

# 创建 XCFramework 目录
mkdir -p "$IOS_DIR/AetherSDK.xcframework/ios-arm64"

# 复制 .a 文件
cp "$PROJECT_DIR/target/aarch64-apple-ios/release/libagent_bindings.a" \
   "$IOS_DIR/AetherSDK.xcframework/ios-arm64/"

echo "✅ iOS SDK 构建完成"
echo ""
echo "下一步: 在 Xcode 中打开项目，将 AetherSDK.xcframework 添加到 Frameworks"
