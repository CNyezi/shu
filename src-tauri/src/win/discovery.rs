//! 应用发现：shell:AppsFolder 枚举（Win32 + UWP 统一、显示名本地化，
//! PowerToys Run 同款方案）。COM 每次防御性初始化（S_FALSE 幂等）。
use windows::core::w;
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, IEnumShellItems, IShellItem, SHCreateItemFromParsingName, SIGDN,
    SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
};

use crate::AppEntry;

fn sigdn_string(item: &IShellItem, kind: SIGDN) -> Option<String> {
    unsafe {
        let p = item.GetDisplayName(kind).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as _));
        s
    }
}

pub fn list_apps() -> Vec<AppEntry> {
    let mut out = Vec::new();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let Ok(folder) =
            SHCreateItemFromParsingName::<_, IShellItem>(w!("shell:AppsFolder"), None)
        else {
            return out;
        };
        let Ok(enumerator) = folder.BindToHandler::<_, IEnumShellItems>(None, &BHID_EnumItems)
        else {
            return out;
        };
        loop {
            let mut items: [Option<IShellItem>; 1] = [None];
            let mut fetched = 0u32;
            if enumerator.Next(&mut items, Some(&mut fetched)).is_err() || fetched == 0 {
                break;
            }
            let Some(item) = items[0].take() else { break };
            let (Some(name), Some(parsing)) = (
                sigdn_string(&item, SIGDN_NORMALDISPLAY),
                sigdn_string(&item, SIGDN_PARENTRELATIVEPARSING),
            ) else {
                continue;
            };
            if name.is_empty() || parsing.is_empty() || super::logic::is_noise_entry(&name) {
                continue;
            }
            out.push(AppEntry {
                name,
                path: format!("shell:AppsFolder\\{parsing}"),
                pinyin: None,
                initials: None,
            });
        }
    }
    out
}
