#!/bin/bash
# R-M1: macOS universal .dylib 构建脚本
# 产出: target/universal/libaether.dylib (aarch64 + x86_64)
# 前置: macOS + Xcode CLT + rustup target add aarch64-apple-darwin x86_64-apple-darwin
set -e

echo "=== 构建 macOS universal dylib ==="

# 编译 ARM64
echo "--- aarch64-apple-darwin ---"
cargo build -p agent-bindings --release --target aarch64-apple-darwin --no-default-features --features cli

# 编译 x86_64
echo "--- x86_64-apple-darwin ---"
cargo build -p agent-bindings --release --target x86_64-apple-darwin --no-default-features --features cli

# 创建 universal binary 输出目录
UNIVERSAL_DIR="target/universal"
mkdir -p "$UNIVERSAL_DIR"

# lipo 合并
echo "--- lipo 合并 ---"
lipo -create \
    "target/aarch64-apple-darwin/release/libagent_bindings.dylib" \
    "target/x86_64-apple-darwin/release/libagent_bindings.dylib" \
    -output "$UNIVERSAL_DIR/libaether.dylib"

# 验证
echo "--- 验证 ---"
file "$UNIVERSAL_DIR/libaether.dylib"
ls -lh "$UNIVERSAL_DIR/libaether.dylib"

echo ""
echo "✅ Universal dylib: $UNIVERSAL_DIR/libaether.dylib"
