//! ShellExecuteW 包装：应用（含 shell:AppsFolder\AUMID）、URL、路径统一入口。
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// ShellExecuteW 默认动词。返回值 ≤32 为错误码。
pub fn shell_open(target: &str) -> Result<(), String> {
    let w_target = wide(target);
    let ret = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            PCWSTR(w_target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if ret.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute 失败（{}）：{}", ret.0 as isize, target))
    }
}
