//! Everything 集成：SDK（Everything64.dll，MIT）动态加载、客户端生命周期、服务安装。
//! SDK 的查询状态是进程全局的，用互斥锁串行化查询。
//! ponytail: 只绑 7 个 SDK 函数，够 everything_search 用；要更多字段时再加。
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// EVERYTHING_ERROR_IPC：没有运行中的 Everything 实例。
const ERR_IPC: u32 = 2;

type SetSearchW = unsafe extern "system" fn(*const u16);
type SetMax = unsafe extern "system" fn(u32);
type QueryW = unsafe extern "system" fn(i32) -> i32;
type GetNumResults = unsafe extern "system" fn() -> u32;
type GetResultFullPathNameW = unsafe extern "system" fn(u32, *mut u16, u32) -> u32;
type IsFolderResult = unsafe extern "system" fn(u32) -> i32;
type GetLastError = unsafe extern "system" fn() -> u32;

struct Sdk {
    _lib: libloading::Library,
    set_search: libloading::os::windows::Symbol<SetSearchW>,
    set_max: libloading::os::windows::Symbol<SetMax>,
    query: libloading::os::windows::Symbol<QueryW>,
    num_results: libloading::os::windows::Symbol<GetNumResults>,
    full_path: libloading::os::windows::Symbol<GetResultFullPathNameW>,
    is_folder: libloading::os::windows::Symbol<IsFolderResult>,
    last_error: libloading::os::windows::Symbol<GetLastError>,
}

unsafe impl Send for Sdk {}
unsafe impl Sync for Sdk {}

fn sdk(dll_dir: &Path) -> Result<&'static Sdk, String> {
    static SDK: OnceLock<Result<Sdk, String>> = OnceLock::new();
    SDK.get_or_init(|| unsafe { load(dll_dir) })
        .as_ref()
        .map_err(|e| e.clone())
}

unsafe fn load(dir: &Path) -> Result<Sdk, String> {
    let lib = libloading::Library::new(dir.join("Everything64.dll"))
        .map_err(|e| format!("加载 Everything64.dll 失败：{e}"))?;
    macro_rules! sym {
        ($name:expr, $t:ty) => {
            lib.get::<$t>($name).map_err(|e| e.to_string())?.into_raw()
        };
    }
    Ok(Sdk {
        set_search: sym!(b"Everything_SetSearchW\0", SetSearchW),
        set_max: sym!(b"Everything_SetMax\0", SetMax),
        query: sym!(b"Everything_QueryW\0", QueryW),
        num_results: sym!(b"Everything_GetNumResults\0", GetNumResults),
        full_path: sym!(b"Everything_GetResultFullPathNameW\0", GetResultFullPathNameW),
        is_folder: sym!(b"Everything_IsFolderResult\0", IsFolderResult),
        last_error: sym!(b"Everything_GetLastError\0", GetLastError),
        _lib: lib,
    })
}

static QUERY_LOCK: Mutex<()> = Mutex::new(());

pub fn is_ipc_error(e: &str) -> bool {
    e == "EVERYTHING_ERR_2"
}

pub fn search(dll_dir: &Path, query: &str, max: u32) -> Result<Vec<crate::FileHit>, String> {
    let sdk = sdk(dll_dir)?;
    let _guard = QUERY_LOCK.lock().unwrap();
    unsafe {
        let w: Vec<u16> = query.encode_utf16().chain(std::iter::once(0)).collect();
        (sdk.set_search)(w.as_ptr());
        (sdk.set_max)(max.max(1));
        if (sdk.query)(1) == 0 {
            return Err(format!("EVERYTHING_ERR_{}", (sdk.last_error)()));
        }
        let n = (sdk.num_results)();
        let mut out = Vec::with_capacity(n as usize);
        let mut buf = vec![0u16; 4096];
        for i in 0..n {
            let len = (sdk.full_path)(i, buf.as_mut_ptr(), buf.len() as u32);
            if len == 0 {
                continue;
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            let name = Path::new(&path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            out.push(crate::FileHit {
                name,
                path,
                is_folder: (sdk.is_folder)(i) != 0,
            });
        }
        Ok(out)
    }
}

fn spawn_client(dir: &Path) -> Result<(), String> {
    Command::new(dir.join("Everything.exe"))
        .arg("-startup")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("启动 Everything 失败：{e}"))?;
    Ok(())
}

/// 确保有可通信的 Everything 实例：优先复用用户已在跑的，否则拉起捆绑客户端。
pub fn ensure_client(dir: &Path) -> Result<(), String> {
    match search(dir, "", 1) {
        Ok(_) => Ok(()),
        Err(e) if is_ipc_error(&e) => {
            spawn_client(dir)?;
            for _ in 0..20 {
                std::thread::sleep(Duration::from_millis(200));
                if search(dir, "", 1).is_ok() {
                    return Ok(());
                }
            }
            Err("Everything 客户端启动超时".into())
        }
        Err(e) => Err(e),
    }
}

/// Everything 服务在则 MFT 直读可用（判定索引能力的确定性代理）。
pub fn service_installed() -> bool {
    Command::new("sc")
        .args(["query", "Everything"])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// UAC 提权安装 Everything 服务，然后重启客户端接上服务索引。
pub fn install_service(dir: &Path) -> Result<(), String> {
    let exe = dir.join("Everything.exe");
    let script = format!(
        "$p = Start-Process -FilePath '{}' -ArgumentList '-install-service' -Verb RunAs -Wait -PassThru; exit $p.ExitCode",
        exe.to_string_lossy()
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("已取消".into());
    }
    let _ = Command::new(&exe)
        .arg("-exit")
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    std::thread::sleep(Duration::from_millis(500));
    spawn_client(dir)?;
    std::thread::sleep(Duration::from_millis(800));
    Ok(())
}
