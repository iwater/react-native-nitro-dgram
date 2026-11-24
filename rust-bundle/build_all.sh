#!/bin/bash
# build_all.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🚀 Building React Native Rust UDP Client..."

AUTO_CONFIRM=false
while getopts "y" opt; do
  case $opt in
    y)
      AUTO_CONFIRM=true
      echo "🤖 Auto-confirm enabled (-y)"
      ;;
    \?)
      echo "Invalid option: -$OPTARG" >&2
      exit 1
      ;;
  esac
done

echo "🧹 Cleaning previous builds..."
cargo clean

echo "🔧 Building Rust library..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Failed to build Rust library"
    exit 1
fi

echo "✅ Rust library built successfully"

# iOS Build
if [ "$AUTO_CONFIRM" = true ]; then
    REPLY="y"
else
    read -p "Do you want to build iOS libraries? (y/n): " -n 1 -r
    echo
fi
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "📱 Building iOS libraries..."
    mkdir -p target/xcframework
    
    IOS_TARGETS=("x86_64-apple-ios" "aarch64-apple-ios-sim" "aarch64-apple-ios")
    
    for target in "${IOS_TARGETS[@]}"; do
        echo "  Building for $target..."
        cargo build --target "$target" --release
        
        if [ $? -ne 0 ]; then
            echo "❌ Failed to build for $target"
            exit 1
        fi
    done
    
    # ===== 动态库构建 (Dynamic Framework) =====
    
    # 创建模拟器通用动态库
    echo "  Creating universal simulator dynamic library..."
    mkdir -p target/universal/ios-sim
    xcrun lipo -create \
        target/x86_64-apple-ios/release/librust_c_udp.dylib \
        target/aarch64-apple-ios-sim/release/librust_c_udp.dylib \
        -output target/universal/ios-sim/librust_c_udp.dylib
    
    # 创建设备动态库目录
    echo "  Preparing device dynamic library..."
    mkdir -p target/universal/ios-device
    cp target/aarch64-apple-ios/release/librust_c_udp.dylib \
       target/universal/ios-device/librust_c_udp.dylib
       
    # 设置正确的 install_name
    echo "  Setting install names..."
    install_name_tool -id @rpath/RustCUdp.framework/RustCUdp \
        target/universal/ios-sim/librust_c_udp.dylib
    install_name_tool -id @rpath/RustCUdp.framework/RustCUdp \
        target/universal/ios-device/librust_c_udp.dylib
        
    # 创建 Framework 结构
    echo "  Creating Framework structure..."
    FRAMEWORK_SIM="target/universal/ios-sim/RustCUdp.framework"
    FRAMEWORK_DEVICE="target/universal/ios-device/RustCUdp.framework"
    
    # 清理旧的 Framework
    rm -rf "$FRAMEWORK_SIM" "$FRAMEWORK_DEVICE"
    
    # 创建模拟器 Framework
    mkdir -p "$FRAMEWORK_SIM/Headers"
    cp target/universal/ios-sim/librust_c_udp.dylib "$FRAMEWORK_SIM/RustCUdp"
    
    # 创建设备 Framework
    mkdir -p "$FRAMEWORK_DEVICE/Headers"
    cp target/universal/ios-device/librust_c_udp.dylib "$FRAMEWORK_DEVICE/RustCUdp"
    
    # 创建 Info.plist for Frameworks
    cat > "$FRAMEWORK_SIM/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>RustCUdp</string>
    <key>CFBundleIdentifier</key>
    <string>com.iwater.RustCUdp</string>
    <key>CFBundleName</key>
    <string>RustCUdp</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>MinimumOSVersion</key>
    <string>13.0</string>
</dict>
</plist>
PLIST
    cp "$FRAMEWORK_SIM/Info.plist" "$FRAMEWORK_DEVICE/Info.plist"
    
    # 生成 XCFramework
    echo "  Creating XCFramework..."
    rm -rf target/xcframework/RustCUdp.xcframework
    
    xcodebuild -create-xcframework \
        -framework "$FRAMEWORK_SIM" \
        -framework "$FRAMEWORK_DEVICE" \
        -output target/xcframework/RustCUdp.xcframework
    
    if [ $? -eq 0 ]; then
        echo "✅ iOS XCFramework built successfully!"
    else
        echo "❌ Failed to create XCFramework"
        exit 1
    fi
fi

# Android Build
if [ "$AUTO_CONFIRM" = true ]; then
    REPLY="y"
else
    read -p "Do you want to build Android libraries? (y/n): " -n 1 -r
    echo
fi
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "🤖 Building Android libraries..."
    
    ANDROID_TARGETS=(
        "aarch64-linux-android"
        "armv7-linux-androideabi" 
        "i686-linux-android"
        "x86_64-linux-android"
    )
    
    for target in "${ANDROID_TARGETS[@]}"; do
        echo "  Building for $target..."
        RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384" cargo ndk --target "$target" --platform 21 build --release
        
        if [ $? -ne 0 ]; then
            echo "❌ Failed to build for $target"
            exit 1
        fi
    done
    
    echo "✅ Android libraries built successfully!"
fi

echo "🧪 Running Rust tests..."
cargo test -- --test-threads=1 --nocapture

echo ""
echo "🎉 Build completed for RustCUdp!"
