//! CF_HDROP 文件列表读取 + 位图格式探测（只探格式不解码，对齐 macOS 版语义）。
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
};
use windows::Win32::System::Ole::{CF_BITMAP, CF_DIB, CF_HDROP};
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

pub fn read_files() -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        if OpenClipboard(None).is_err() {
            return out;
        }
        if let Ok(handle) = GetClipboardData(CF_HDROP.0 as u32) {
            let hdrop = HDROP(handle.0);
            let count = DragQueryFileW(hdrop, u32::MAX, None);
            for i in 0..count {
                let len = DragQueryFileW(hdrop, i, None);
                if len == 0 {
                    continue;
                }
                let mut buf = vec![0u16; (len + 1) as usize];
                let got = DragQueryFileW(hdrop, i, Some(&mut buf));
                if got > 0 {
                    out.push(String::from_utf16_lossy(&buf[..got as usize]));
                }
            }
        }
        let _ = CloseClipboard();
    }
    out
}

pub fn image_present() -> bool {
    unsafe {
        IsClipboardFormatAvailable(CF_DIB.0 as u32).is_ok()
            || IsClipboardFormatAvailable(CF_BITMAP.0 as u32).is_ok()
    }
}
