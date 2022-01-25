#![crate_type = "cdylib"]
#![cfg(all(target_os = "linux"))]
#![feature(once_cell)]

use std::error::Error;
use std::ffi::{c_void, CStr, OsString};
use std::io::Read;
use std::lazy::OnceCell;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use libc::{c_char, c_int};

use sabinokaku_common::prelude::*;

struct LinuxConfigSearchPath;

impl ConfigSearchPath for LinuxConfigSearchPath {
    fn get_module_path() -> Option<PathBuf> {
        let mut dlinfo = MaybeUninit::<libc::Dl_info>::uninit();
        let slice = unsafe {
            if libc::dladdr(thunked_main as *const c_void, dlinfo.as_mut_ptr()) == 0 {
                return None;
            }
            let dlinfo = dlinfo.assume_init();
            let fname = dlinfo.dli_fname;
            CStr::from_ptr(fname).to_owned()
        };
        let os_str = OsString::from_vec(slice.into_bytes());
        PathBuf::from(os_str).canonicalize().ok()
    }
}

/// Called when the DLL is attached to the process.
fn main() -> Result<i32, Box<dyn Error>> {
    let cfg_path = LinuxConfigSearchPath::search_for_config()?;
    let mut file = std::fs::File::open(&cfg_path)?;
    let mut cfg_string = String::new();
    file.read_to_string(&mut cfg_string)?;
    let config = LoadConfig::try_parse(cfg_path, cfg_string)?;
    Ok(init_clr(config)?)
}

type FnMain = unsafe extern "system" fn(c_int, *mut *mut c_char, *mut *mut c_char) -> c_int;
type FnVoid = unsafe extern "system" fn();
type FnLibcStartMain = unsafe extern "system" fn(FnMain, c_int, *mut *mut c_char, FnMain, FnVoid, FnVoid, *mut c_void) -> c_int;

const LIBC_START_MAIN: &'static [u8] = b"__libc_start_main\0";

// This is really bad...
static mut SAVED_MAIN: OnceCell<FnMain> = OnceCell::new();

unsafe extern "system" fn thunked_main(argc: c_int, argv: *mut *mut c_char, envp: *mut *mut c_char) -> c_int {
    // We don't wait for the thread. This is consistent with windows behaviour.
    std::thread::spawn(|| {
        match main() {
            Ok(i) => {
                i as u32
            }
            Err(e) => {
                eprintln!("Error occurred when injecting CLR: {}", e);
                1 as u32
            }
        }
    });

    let ret = if let Some(real_main) = SAVED_MAIN.get() {
        real_main(argc, argv, envp)
    } else {
        0
    };

    ret
}

#[no_mangle]
pub unsafe extern "system" fn __libc_start_main(
    main: FnMain,
    argc: c_int,
    argv: *mut *mut c_char,
    init: FnMain,
    fini: FnVoid,
    rtld_fini: FnVoid,
    stack_end: *mut c_void,
) -> c_int {
    let origin_start: FnLibcStartMain = std::mem::transmute(libc::dlsym(libc::RTLD_NEXT, LIBC_START_MAIN.as_ptr() as *const c_char));
    SAVED_MAIN.get_or_init(move || main);

    return origin_start(thunked_main, argc, argv, init, fini, rtld_fini, stack_end);
}
