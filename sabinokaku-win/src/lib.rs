#![crate_type = "cdylib"]
#![cfg(all(target_os = "windows"))]

use std::ffi::{c_void, OsString};
use std::error::Error;
use std::io::Read;
use std::env::current_exe;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use winapi::shared::minwindef::*;
use winapi::shared::ntdef::*;
use winapi::um::handleapi::CloseHandle;

use winapi::um::libloaderapi::{DisableThreadLibraryCalls, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                               GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                               GetModuleFileNameW, GetModuleHandleExW};

use winapi::um::winnt::DLL_PROCESS_ATTACH;
use winapi::um::minwinbase::LPSECURITY_ATTRIBUTES;
use winapi::um::processthreadsapi::CreateThread;

use sabinokaku_common::prelude::*;

unsafe fn get_module_path() -> Option<PathBuf> {
    let mut module_handle: HMODULE = std::ptr::null_mut();
    if GetModuleHandleExW(
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        DllMain as LPCWSTR, &mut module_handle
    ) == 0 {
        return None
    }
    let mut v: Vec<u16> = Vec::with_capacity(MAX_PATH);

    let size = GetModuleFileNameW(module_handle, v.as_mut_ptr(), MAX_PATH as DWORD);
    if size == 0 {
        return None
    }

    v.set_len(size as usize);
    let os_str = OsString::from_wide(&v);
    PathBuf::from(os_str).canonicalize().ok()
}

fn search_for_config() -> Result<PathBuf, Box<dyn Error>> {
    let module_parent = unsafe { get_module_path() };
    if let Some(Some(mut kaku_path)) = module_parent.map(|s| s.parent().map(PathBuf::from)) {
        kaku_path.push("kaku.co");
        if kaku_path.exists() {
            return Ok(kaku_path)
        }
    }

    if let Ok(Some(mut kaku_path)) = current_exe().map(|s| s.parent().map(PathBuf::from)) {
        kaku_path.push("kaku.co");
        if kaku_path.exists() {
            return Ok(kaku_path)
        }
    }

    Err(Box::new(ConfigError::MissingConfig))
}

/// Called when the DLL is attached to the process.
fn main() -> Result<i32, Box<dyn Error>> {
    let cfg_path = search_for_config()?;
    let mut file = std::fs::File::open(&cfg_path)?;
    let mut cfg_string = String::new();
    file.read_to_string(&mut cfg_string)?;
    let config = LoadConfig::try_parse(cfg_path, cfg_string)?;
    Ok(sabinokaku_common::init_clr(config)?)
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn thunk_thread_main(_: *mut c_void) -> u32 {
    match main() {
        Ok(i) => i as u32,
        Err(e) => {
            winapi::um::consoleapi::AllocConsole();
            println!("Error occurred when injecting CLR: {}", e);
            1
        }
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
        CloseHandle(CreateThread(0 as LPSECURITY_ATTRIBUTES, 0, Some(thunk_thread_main),
                     0 as LPVOID, 0, 0 as LPDWORD));
    }
    winapi::shared::minwindef::TRUE
}
