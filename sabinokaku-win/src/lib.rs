#![crate_type = "cdylib"]
#![cfg(all(target_os = "windows"))]

use std::error::Error;
use std::ffi::OsString;
use std::io::Read;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use winapi::shared::minwindef::*;
use winapi::shared::ntdef::*;
use winapi::um::libloaderapi::{DisableThreadLibraryCalls, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                               GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                               GetModuleFileNameW, GetModuleHandleExW};
use winapi::um::winnt::DLL_PROCESS_ATTACH;

use sabinokaku_common::prelude::*;

struct WindowsConfigSearchPath;
impl ConfigSearchPath for WindowsConfigSearchPath {
    fn get_module_path() -> Option<PathBuf> {
        let mut module_handle: HMODULE = std::ptr::null_mut();
        unsafe {
            if GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                DllMain as LPCWSTR, &mut module_handle
            ) == 0 {
                return None
            }
        }
        let mut v: Vec<u16> = Vec::with_capacity(MAX_PATH);

        unsafe {
            let size =
                GetModuleFileNameW(module_handle, v.as_mut_ptr(), MAX_PATH as DWORD);
            if size == 0 {
                return None
            }
            v.set_len(size as usize);
        }
        let os_str = OsString::from_wide(&v);
        PathBuf::from(os_str).canonicalize().ok()
    }
}

/// Called when the DLL is attached to the process.
fn main() -> Result<i32, Box<dyn Error>> {
    let cfg_path = WindowsConfigSearchPath::search_for_config()?;
    let mut file = std::fs::File::open(&cfg_path)?;
    let mut cfg_string = String::new();
    file.read_to_string(&mut cfg_string)?;
    let config = LoadConfig::try_parse(cfg_path, cfg_string)?;
    Ok(sabinokaku_common::init_clr(config)?)
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllMain(
    module: HINSTANCE,
    call_reason: DWORD,
    _reserved: LPVOID,
) -> BOOL {
    DisableThreadLibraryCalls(module);

    if call_reason == DLL_PROCESS_ATTACH {
        std::thread::spawn(|| {
            match main() {
                Ok(i) => i as u32,
                Err(e) => {
                    winapi::um::consoleapi::AllocConsole();
                    println!("Error occurred when injecting CLR: {}", e);
                    1
                }
            }
        });
    }
    winapi::shared::minwindef::TRUE
}
