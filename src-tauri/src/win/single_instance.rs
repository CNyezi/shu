//! 单实例：命名互斥量判重；第二实例经命名事件唤醒主实例窗口后退出。
//! 手写而非引插件——单实例是宿主的分内事（grilling 决策 2026-07-16）。
use windows::core::w;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE,
    INFINITE,
};

/// 尝试成为唯一实例。已有实例 → 发唤醒信号并返回 false（调用方应直接 return）。
pub fn acquire_or_wake() -> bool {
    unsafe {
        // 互斥量句柄故意不关——进程存活期间持有即锁。
        let _mutex = CreateMutexW(None, true, w!("shu-single-instance-mutex"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if let Ok(ev) = OpenEventW(EVENT_MODIFY_STATE, false, w!("shu-single-instance-wake")) {
                let _ = SetEvent(ev);
                let _ = CloseHandle(ev);
            }
            return false;
        }
        true
    }
}

/// 主实例：后台线程等唤醒事件，收到即回调（回调内做 show+focus，Tauri 窗口方法线程安全）。
pub fn spawn_wake_listener(on_wake: impl Fn() + Send + 'static) {
    unsafe {
        let Ok(event) = CreateEventW(None, false, false, w!("shu-single-instance-wake")) else {
            return;
        };
        let raw = event.0 as isize;
        std::thread::spawn(move || {
            let h = HANDLE(raw as _);
            loop {
                if unsafe { WaitForSingleObject(h, INFINITE) } != WAIT_OBJECT_0 {
                    break;
                }
                on_wake();
            }
        });
    }
}
