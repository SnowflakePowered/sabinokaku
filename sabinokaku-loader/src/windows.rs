#![cfg(all(target_os = "windows"))]

use std::ffi::{OsStr, OsString};
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
        if unsafe {
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                DllMain as LPCWSTR, &mut module_handle,
            )
        } == 0 {
            return None;
        }

        let module_fname = unsafe {
            let mut v: Vec<u16> = Vec::with_capacity(MAX_PATH);
            let size =
                GetModuleFileNameW(module_handle, v.as_mut_ptr(), MAX_PATH as DWORD);
            if size == 0 {
                return None;
            }
            v.set_len(size as usize);
            v
        };

        let os_str = OsString::from_wide(&module_fname);
        PathBuf::from(os_str).canonicalize().ok()
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    module: HINSTANCE,
    call_reason: DWORD,
    _reserved: LPVOID,
) -> BOOL {
    unsafe { DisableThreadLibraryCalls(module); }
    if call_reason == DLL_PROCESS_ATTACH {
        std::thread::spawn(|| {
            let config = match crate::get_config() {
                Ok(config) => config,
                Err(e) => {
                    unsafe { winapi::um::consoleapi::AllocConsole(); }
                    eprintln!("[dllmain_inject] Error occurred when parsing config: {}", e);
                    return 1
                }
            };
            
            #[cfg(feature = "vulkan")] {
                if let Some(true) = std::env::var_os("ENABLE_SABINOKAKU_VULKAN").map(|s| s == OsStr::new("1")) {
                    eprintln!("[dllmain_inject] Vulkan env enabled.");
                    if config.vulkan().is_some() {
                        eprintln!("[dllmain_inject] Vulkan config detected, disabling load entry.");
                        return 0
                    }
                }
            }

            match crate::boot_clr::<()>(config, None) {
                Ok(i) => i as u32,
                Err(e) => {
                    unsafe { winapi::um::consoleapi::AllocConsole(); }
                    eprintln!("[dllmain_inject] Error occurred when injecting CLR: {}", e);
                    1
                }
            }
        });
    }
    winapi::shared::minwindef::TRUE
}
