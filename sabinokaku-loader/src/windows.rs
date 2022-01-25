#![cfg(all(target_os = "windows"))]
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use winapi::shared::minwindef::*;
use winapi::shared::ntdef::*;
use winapi::um::libloaderapi::{DisableThreadLibraryCalls, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                               GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                               GetModuleFileNameW, GetModuleHandleExW};
use winapi::um::winnt::DLL_PROCESS_ATTACH;

use sabinokaku_common::config::ConfigSearchPath;

pub struct WindowsConfigSearchPath;
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
            match crate::main() {
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
