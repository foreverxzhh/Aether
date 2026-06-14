//! C API 导出（供 C# / Python / Go 等语言 P/Invoke 调用）

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;
use agent_core::AIAgent;
use agent_core::config::AgentConfigBuilder;

/// Agent 句柄（线程安全）
struct AgentHandle(Mutex<Option<AIAgent>>);

/// 创建 Agent
#[no_mangle]
pub extern "C" fn aether_create(
    provider: *const c_char,
    model: *const c_char,
    api_key: *const c_char,
) -> *mut AgentHandle {
    let provider = unsafe { CStr::from_ptr(provider) }.to_string_lossy();
    let model = unsafe { CStr::from_ptr(model) }.to_string_lossy();
    let api_key = if api_key.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(api_key) }.to_string_lossy().to_string()
    };

    let mut builder = AgentConfigBuilder::new()
        .provider(provider.as_ref())
        .model(model.as_ref());
    if !api_key.is_empty() {
        builder = builder.api_key(api_key.as_str());
    }

    let agent = AIAgent::new(builder.build());
    Box::into_raw(Box::new(AgentHandle(Mutex::new(Some(agent)))))
}

/// 初始化 LLM 供应商
#[no_mangle]
pub extern "C" fn aether_init_model(handle: *mut AgentHandle) -> i32 {
    let agent = unsafe { &*handle };
    let mut guard = agent.0.lock().unwrap();
    if let Some(ref mut a) = *guard {
        let rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(a.init_model()) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// 对话（返回 JSON 字符串，需调用 aether_free_string 释放）
#[no_mangle]
pub extern "C" fn aether_chat(
    handle: *mut AgentHandle,
    message: *const c_char,
) -> *mut c_char {
    let agent = unsafe { &*handle };
    let msg = unsafe { CStr::from_ptr(message) }.to_string_lossy().to_string();

    let result = {
        let guard = agent.0.lock().unwrap();
        if let Some(ref a) = *guard {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(a.chat(&msg))
        } else {
            Err(agent_core::AetherError::ConfigError("Agent 已销毁".into()))
        }
    };

    let json = match result {
        Ok(reply) => format!(r#"{{"success":true,"reply":"{}"}}"#, reply.replace('"', r#"\""#)),
        Err(e) => format!(r#"{{"success":false,"error":"{}"}}"#, e.to_string().replace('"', r#"\""#)),
    };

    CString::new(json).unwrap_or_default().into_raw()
}

/// 释放字符串
#[no_mangle]
pub extern "C" fn aether_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { let _ = CString::from_raw(s); }
    }
}

/// 销毁 Agent
#[no_mangle]
pub extern "C" fn aether_destroy(handle: *mut AgentHandle) {
    if !handle.is_null() {
        unsafe {
            let agent = Box::from_raw(handle);
            let mut guard = agent.0.lock().unwrap();
            *guard = None; // 触发 drop
        }
    }
}
