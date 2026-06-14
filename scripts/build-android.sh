#!/bin/bash
# 构建 Aether Android SDK
#
# 前置条件:
#   1. 安装 Rust:         rustup target add aarch64-linux-android
#   2. 安装 Android NDK:  从 Android Studio 下载，或直接下载 NDK
#   3. 设置环境变量:       export ANDROID_NDK_HOME=/path/to/ndk
#   4. (Windows 需加)      export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/bin:$PATH"
#
# 运行:
#   bash scripts/build-android.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR/.."
ANDROID_DIR="$PROJECT_DIR/sdks/android"

# 颜色
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}🔨 构建 Aether Android SDK${NC}"
echo ""

# 检查 ANDROID_NDK_HOME
if [ -z "$ANDROID_NDK_HOME" ]; then
    echo -e "${YELLOW}⚠️  ANDROID_NDK_HOME 未设置${NC}"
    echo "   请安装 NDK 并设置环境变量:"
    echo "   export ANDROID_NDK_HOME=\"/path/to/android-ndk\""
    echo ""
    echo "   然后确保 NDK 工具链在 PATH 中:"
    echo '   export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/bin:$PATH"'
    echo ""
    echo -e "${YELLOW}   ⚠️  跳过 native 库编译，只生成 Kotlin 绑定${NC}"
    echo ""
    NDK_AVAILABLE=false
else
    NDK_AVAILABLE=true
    # 将 NDK 工具链加入 PATH
    TOOLCHAIN_DIR="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt"
    if [ -d "$TOOLCHAIN_DIR/windows-x86_64" ]; then
        export PATH="$TOOLCHAIN_DIR/windows-x86_64/bin:$PATH"
    elif [ -d "$TOOLCHAIN_DIR/linux-x86_64" ]; then
        export PATH="$TOOLCHAIN_DIR/linux-x86_64/bin:$PATH"
    elif [ -d "$TOOLCHAIN_DIR/darwin-x86_64" ]; then
        export PATH="$TOOLCHAIN_DIR/darwin-x86_64/bin:$PATH"
    fi
fi

# 确保 Rust 目标已安装
echo "📋 检查 Rust Android 目标..."
for target in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
    if rustup target list --installed | grep -q "$target"; then
        echo "  ✅ $target"
    else
        echo "  ⚠️  安装 $target ..."
        rustup target add "$target"
    fi
done
echo ""

# 重新生成 Kotlin 绑定
echo "📄 生成 Kotlin 绑定..."
mkdir -p "$ANDROID_DIR/src/main/java"
export PATH="$HOME/.cargo/bin:\$PATH"
uniffi-bindgen generate "$PROJECT_DIR/agent-bindings/src/agent.udl" \
    --language kotlin \
    --out-dir "$ANDROID_DIR/src/main/java" 2>/dev/null
echo "  ✅ 绑定生成完成"
echo ""

# 编译 native 库（仅当 NDK 可用时）
if [ "$NDK_AVAILABLE" = true ]; then
    echo "🔧 编译 native 库..."

    declare -A TARGET_MAP
    TARGET_MAP["aarch64-linux-android"]="arm64-v8a"
    TARGET_MAP["armv7-linux-androideabi"]="armeabi-v7a"
    TARGET_MAP["x86_64-linux-android"]="x86_64"

    for target in "${!TARGET_MAP[@]}"; do
        jni_dir="${TARGET_MAP[$target]}"
        echo "  → 构建 $target ..."
        cargo build -p agent-bindings --no-default-features \
            --target "$target" --release 2>&1 | tail -2

        # 复制 .so 到 JNI 目录
        mkdir -p "$ANDROID_DIR/src/main/jniLibs/$jni_dir"
        cp "$PROJECT_DIR/target/$target/release/libagent_bindings.so" \
           "$ANDROID_DIR/src/main/jniLibs/$jni_dir/libaether.so" 2>/dev/null && \
        echo "    ✅ $jni_dir/libaether.so" || \
        echo "    ⚠️  $target .so 未找到"
    done
    echo ""
    echo -e "${GREEN}✅ Native 库编译完成${NC}"
else
    echo -e "${YELLOW}⏳ 跳过 native 编译（需要 Android NDK）${NC}"
fi

echo ""
echo "📦 Android SDK 位置: $ANDROID_DIR"
echo ""
echo "下一步:"
echo "  1. 用 Android Studio 打开 $ANDROID_DIR"
echo "  2. Build → Build Bundle(s) / APK"
echo "  3. 在 build/outputs/aar/ 找到 AAR 文件"
