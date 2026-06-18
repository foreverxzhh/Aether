#!/bin/bash
# R-M3: 构建 macOS demo
# 前置: 先运行 scripts/build-macos.sh 产出 libaether.dylib
set -e
UNIVERSAL="../../target/universal"
swiftc -o macos-demo main.swift \
    -I "$UNIVERSAL" \
    -L "$UNIVERSAL" \
    -laether
echo "✅ macos-demo 构建完成"
echo "运行: DYLD_LIBRARY_PATH=$UNIVERSAL ./macos-demo"
