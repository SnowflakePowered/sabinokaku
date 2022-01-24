#![crate_type = "cdylib"]

use std::ffi::{c_void, CString, OsStr};
use std::mem::MaybeUninit;
use std::os::windows::prelude::OsStrExt;
use std::{error::Error, mem::size_of};
use std::ptr::null_mut;
use winapi::shared::d3d9::*;
use winapi::shared::d3d9types::*;
use winapi::shared::minwindef::*;
use winapi::shared::ntdef::*;
use winapi::shared::windef::*;

use winapi::um::libloaderapi::{DisableThreadLibraryCalls, GetProcAddress};
use winapi::um::winuser::{CreateWindowExW, RegisterClassExW, WS_OVERLAPPEDWINDOW, DestroyWindow, UnregisterClassW};
use winapi::um::{
    libloaderapi::GetModuleHandleW,
    winnt::DLL_PROCESS_ATTACH,
    winuser::{DefWindowProcW, CS_HREDRAW, CS_VREDRAW, WNDCLASSEXW},
};

use winapi::um::errhandlingapi::GetLastError;
use winapi::um::minwinbase::LPSECURITY_ATTRIBUTES;
use winapi::um::processthreadsapi::CreateThread;
use winapi::um::winnt::LPCWSTR;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

fn to_wchar(str: &str) -> Vec<u16> {
    str.encode_utf16().chain(Some(0)).collect()
}

fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
    let module = to_wchar(module);
    let symbol = CString::new(symbol).unwrap();
    unsafe {
        let handle = GetModuleHandleW(module.as_ptr());
        match GetProcAddress(handle, symbol.as_ptr()) as usize {
            0 => None,
            n => Some(n),
        }
    }
}

/// Called when the DLL is attached to the process.
unsafe fn main() -> Result<(), Box<dyn Error>> {
    // Retrieve an absolute address of `MessageBoxW`. This is required for
    // libraries due to the import address table. If `MessageBoxW` would be
    // provided directly as the target, it would only hook this DLL's
    // `MessageBoxW`. Using the method below an absolute address is retrieved
    // instead, detouring all invocations of `MessageBoxW` in the active process.
    println!("Injecting...");

    let className = to_wchar("sabitsuku");
    let windowName = to_wchar("sabitsuku DirectX Window");
    // d3d shit
    let windowClass: WNDCLASSEXW = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(DefWindowProcW),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: GetModuleHandleW(0 as LPCWSTR),
        hIcon: 0 as HICON,
        hCursor: 0 as HCURSOR,
        hbrBackground: 0 as HBRUSH,
        lpszMenuName: 0 as LPCWSTR,
        lpszClassName: className.as_ptr() as LPCWSTR,
        hIconSm: 0 as HICON,
    };

    let reg_class_res = RegisterClassExW(&windowClass);
    let window: HWND = CreateWindowExW(
        0 as DWORD,
        windowClass.lpszClassName,
        windowName.as_ptr() as LPCWSTR,
        WS_OVERLAPPEDWINDOW,
        0,
        0,
        100,
        100,
        0 as HWND,
        0 as HMENU,
        windowClass.hInstance,
        NULL,
    );

    let err = GetLastError();

    let direct3D9 = Direct3DCreate9(D3D_SDK_VERSION);

    let mut d3d9_params: D3DPRESENT_PARAMETERS = D3DPRESENT_PARAMETERS {
        BackBufferWidth: 0,
        BackBufferHeight: 0,
        BackBufferFormat: D3DFMT_UNKNOWN,
        BackBufferCount: 0,
        MultiSampleType: D3DMULTISAMPLE_NONE,
        MultiSampleQuality: 0,
        SwapEffect: D3DSWAPEFFECT_DISCARD,
        hDeviceWindow: window,
        Windowed: 1,
        EnableAutoDepthStencil: 0,
        AutoDepthStencilFormat: D3DFMT_UNKNOWN,
        Flags: 0,
        FullScreen_RefreshRateInHz: 0,
        PresentationInterval: 0,
    };
    let mut device: LPDIRECT3DDEVICE9 = null_mut() as LPDIRECT3DDEVICE9;
    let result = (*direct3D9).CreateDevice(
        D3DADAPTER_DEFAULT,
        D3DDEVTYPE_NULLREF,
        window,
        D3DCREATE_SOFTWARE_VERTEXPROCESSING | D3DCREATE_DISABLE_DRIVER_MANAGEMENT,
        &mut d3d9_params,
        &mut device,
    );
    if result < 0 {
        println!("Failed to create D3D9 device {:x}", result);
        (*direct3D9).Release();
    } else {
      println!("Successfully created D3D9 device");
        (*direct3D9).Release();
        (*device).Release();
    }
    
    DestroyWindow(window);
    UnregisterClassW(windowClass.lpszClassName, windowClass.hInstance);
    // let fnDirect3DCreate9 = get_module_symbol_address("d3d9.dll", "Direct3DCreate9")
    //   .expect("could not find 'Direct3DCreate9' address");
    // let fnDirect3DCreate9 : = mem::transmute(fnDirect3DCreate9);
    Ok(())
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn thunk_thread_main(_: *mut c_void) -> u32 {
    main().is_ok() as u32
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
        winapi::um::consoleapi::AllocConsole();
        CreateThread(0 as LPSECURITY_ATTRIBUTES, 0, Some(thunk_thread_main),
                     0 as LPVOID, 0, 0 as LPDWORD);
    }
    winapi::shared::minwindef::TRUE
}
