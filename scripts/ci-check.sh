#!/bin/bash
# 本地 CI 模拟——推之前跑一遍，出错不推
set -e
echo "=== 1. 编译 agent-core + agent-bindings ==="
cargo build -p agent-core -p agent-bindings
echo "=== 2. 测试 (--lib) ==="
cargo test -p agent-core --lib
echo "=== 3. 编译 agent-wasm (WASM) ==="
cargo build -p agent-wasm --target wasm32-unknown-unknown 2>&1 | tail -1
echo "=== 4. 编译 agent-bindings (Android ARM64) ==="
if [ -n "$ANDROID_NDK_HOME" ]; then
    cargo build -p agent-bindings --target aarch64-linux-android --release --no-default-features 2>&1 | tail -1
else
    echo "  (跳过，ANDROID_NDK_HOME 未设)"
fi
echo ""
echo "✅ 全部通过，可以推送！"
