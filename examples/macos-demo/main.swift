// R-M3: macOS demo — 最小化 Swift CLI 调用 libaether.dylib
// 构建: swiftc -o macos-demo main.swift -I ../../target/universal -L ../../target/universal -laether
// 运行: DYLD_LIBRARY_PATH=../../target/universal ./macos-demo

import Foundation

// C API 声明
@_silgen_name("aether_create")
func aether_create(_ provider: UnsafePointer<CChar>?, _ model: UnsafePointer<CChar>?, _ apiKey: UnsafePointer<CChar>?) -> OpaquePointer?

@_silgen_name("aether_init_model")
func aether_init_model(_ handle: OpaquePointer?) -> Int32

@_silgen_name("aether_chat")
func aether_chat(_ handle: OpaquePointer?, _ message: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("aether_destroy")
func aether_destroy(_ handle: OpaquePointer?)

// R-W3: aether_free_string 释放 Rust 分配的字符串
@_silgen_name("aether_free_string")
func aether_free_string(_ ptr: UnsafeMutablePointer<CChar>?)

func main() {
    let provider = "deepseek"
    let model = "deepseek-v4-flash"
    let apiKey = ProcessInfo.processInfo.environment["DEEPSEEK_API_KEY"] ?? ""

    guard let handle = aether_create(provider, model, apiKey) else {
        print("failed to create agent")
        exit(1)
    }
    defer { aether_destroy(handle) }

    guard aether_init_model(handle) == 0 else {
        print("failed to init model")
        exit(1)
    }

    guard let replyPtr = aether_chat(handle, "你好") else {
        print("chat failed")
        exit(1)
    }
    let reply = String(cString: replyPtr)
    aether_free_string(replyPtr)

    print("Aether: \(reply)")
}

main()
