#![cfg(all(target_os = "linux"))]
use std::ffi::{c_void, CStr, OsString};
use std::lazy::SyncOnceCell;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use libc::{c_char, c_int};

use sabinokaku_common::config::ConfigSearchPath;

pub struct LinuxConfigSearchPath;
impl ConfigSearchPath for LinuxConfigSearchPath {
    fn get_module_path() -> Option<PathBuf> {
        let mut dlinfo = MaybeUninit::<libc::Dl_info>::uninit();
        let module_fname = unsafe {
            if libc::dladdr(thunked_main as *const c_void, dlinfo.as_mut_ptr()) == 0 {
                return None;
            }
            let dlinfo = dlinfo.assume_init();
            let fname = dlinfo.dli_fname;
            CStr::from_ptr(fname).to_owned()
        };
        let os_str = OsString::from_vec(module_fname.into_bytes());
        PathBuf::from(os_str).canonicalize().ok()
    }
}

type FnMain = extern "system" fn(c_int, *mut *mut c_char, *mut *mut c_char) -> c_int;
type FnVoid = extern "system" fn();
type FnLibcStartMain = extern "system" fn(FnMain, c_int, *mut *mut c_char, FnMain, FnVoid, FnVoid, *mut c_void) -> c_int;

const LIBC_START_MAIN: &'static [u8] = b"__libc_start_main\0";

extern "system" fn thunked_main(argc: c_int, argv: *mut *mut c_char, envp: *mut *mut c_char) -> c_int {
    // We don't wait for the thread. This is consistent with windows behaviour.
    std::thread::spawn(|| {
        let config = match crate::get_config() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("[libc_inject] Error occurred when injecting CLR: {}", e);
                return 1
            }
        };

        if let Some(true) = std::env::var_os("ENABLE_SABINOKAKU_VULKAN").map(|s| s == OsStr::new("1")) {
            println!("[libc_inject] Vulkan env enabled.");
            if config.vulkan().is_some() {
                println!("[libc_inject] Vulkan config detected, disabling load entry.");
                return 0
            }
        }

        match crate::boot_clr::<()>(config,None) {
            Ok(i) => {
                i as u32
            }
            Err(e) => {
                eprintln!("[libc_inject] Error occurred when injecting CLR: {}", e);
                1 as u32
            }
        }
    });

    let ret = if let Some(real_main) = SAVED_MAIN.get() {
        real_main(argc, argv, envp)
    } else {
        // this should never happen but might as well exit if somehow we get launched into
        // here without main.
        eprintln!("[libc_inject] No valid main entrypoint to inject into found.");
        1
    };

    ret
}

// We need to save the original main function somehow.
// While this is pretty bad, we can't really pass anything into FnLibcStartMain,
// so this is the next best thing.
//
// SyncOnceCell is marginally safer than OnceCell and doesn't require static mut.
static SAVED_MAIN: SyncOnceCell<FnMain> = SyncOnceCell::new();

#[no_mangle]
pub extern "system" fn __libc_start_main(
    main: FnMain,
    argc: c_int,
    argv: *mut *mut c_char,
    init: FnMain,
    fini: FnVoid,
    rtld_fini: FnVoid,
    stack_end: *mut c_void,
) -> c_int {
    let origin_start: FnLibcStartMain = unsafe {
        std::mem::transmute(libc::dlsym(libc::RTLD_NEXT, LIBC_START_MAIN.as_ptr() as *const c_char))
    };
    SAVED_MAIN.get_or_init(move || main);
    return origin_start(thunked_main, argc, argv, init, fini, rtld_fini, stack_end);
}
