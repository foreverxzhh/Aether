use std::sync::LazyLock;
use tokio::runtime::Runtime;

/// 全局 tokio runtime（懒加载，只创建一次）
/// 用于 C API 和 UniFFI 绑定的同步调用桥接
static GLOBAL_RT: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to create global tokio runtime")
});

pub(crate) fn global_runtime() -> &'static Runtime {
    &GLOBAL_RT
}
