//! C API 导出（供 C# / Python / Go 等语言 P/Invoke 调用）
//! 使用全局 tokio runtime 避免重复创建

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;
use agent_core::AIAgent;
use agent_core::config::AgentConfigBuilder;
use super::runtime::global_runtime;

struct AgentHandle(Mutex<Option<AIAgent>>);

#[no_mangle]
pub extern "C" fn aether_create(
    provider: *const c_char, model: *const c_char, api_key: *const c_char,
) -> *mut AgentHandle {
    let provider = unsafe { CStr::from_ptr(provider) }.to_string_lossy();
    let model = unsafe { CStr::from_ptr(model) }.to_string_lossy();
    let api_key = if api_key.is_null() { String::new() }
        else { unsafe { CStr::from_ptr(api_key) }.to_string_lossy().to_string() };

    let mut builder = AgentConfigBuilder::new().provider(provider.as_ref()).model(model.as_ref());
    if !api_key.is_empty() { builder = builder.api_key(api_key.as_str()); }
    let agent = AIAgent::new(builder.build());
    Box::into_raw(Box::new(AgentHandle(Mutex::new(Some(agent)))))
}

#[no_mangle]
pub extern "C" fn aether_init_model(handle: *mut AgentHandle) -> i32 {
    let a = unsafe { &*handle };
    let mut g = a.0.lock().unwrap();
    match g.as_mut().and_then(|ag| global_runtime().block_on(ag.init_model()).ok()) {
        Some(_) => 0, None => -1
    }
}

#[no_mangle]
pub extern "C" fn aether_chat(handle: *mut AgentHandle, message: *const c_char) -> *mut c_char {
    let a = unsafe { &*handle };
    let msg = unsafe { CStr::from_ptr(message) }.to_string_lossy().to_string();
    let result = {
        let g = a.0.lock().unwrap();
        g.as_ref().map(|ag| global_runtime().block_on(ag.chat(&msg)))
    };
    let json = match result {
        Some(Ok(reply)) => format!(r#"{{"success":true,"reply":{}}}"#, serde_json::json!(reply)),
        Some(Err(e)) => format!(r#"{{"success":false,"error":{}}}"#, serde_json::json!(e.to_string())),
        None => r#"{"success":false,"error":"Agent已销毁"}"#.to_string(),
    };
    CString::new(json).unwrap_or_default().into_raw()
}

#[no_mangle]
pub extern "C" fn aether_free_string(s: *mut c_char) {
    if !s.is_null() { unsafe { let _ = CString::from_raw(s); } }
}

#[no_mangle]
pub extern "C" fn aether_destroy(handle: *mut AgentHandle) {
    if !handle.is_null() {
        unsafe { let a = Box::from_raw(handle); let mut g = a.0.lock().unwrap(); *g = None; }
    }
}
