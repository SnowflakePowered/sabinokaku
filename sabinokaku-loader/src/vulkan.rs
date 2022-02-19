use std::collections::HashMap;
use std::ffi::{c_void, CStr, OsStr};
use std::lazy::{SyncLazy, SyncOnceCell};
use std::sync::RwLock;
use ash::vk;
use ash::vk::{Handle, Result};
use sabinokaku_common::config::{LoadConfig, VulkanEntryPoint};
use std::os::raw::c_char;
use std::thread;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
#[must_use]
pub struct VkLayerNegotiateStructType(pub(crate) i32);
impl VkLayerNegotiateStructType {
    pub const LAYER_NEGOTIATE_INTERFACE_STRUCT: Self = Self(1);
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
#[must_use]
pub struct VkLayerFunction(pub(crate) i32);
impl VkLayerFunction {
    pub const VK_LAYER_FUNCTION_LINK: Self = Self(0);
    #[allow(dead_code)]
    pub const VK_LAYER_FUNCTION_DATA_CALLBACK: Self = Self(1);
}

// typedef PFN_vkVoidFunction (VKAPI_PTR *PFN_GetPhysicalDeviceProcAddr)(VkInstance instance, const char* pName);
#[allow(non_camel_case_types)]
type PFN_GetPhysicalDeviceProcAddr = vk::PFN_vkGetInstanceProcAddr;

#[repr(C)]
pub struct VkNegotiateLayerInterface {
    pub s_type: VkLayerNegotiateStructType,
    pub p_next: *const c_void,
    pub loader_layer_interface_version: u32,
    pub pfn_get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
    pub pfn_get_device_proc_addr:  vk::PFN_vkGetDeviceProcAddr,
    pub pfn_get_physical_device_proc_addr: Option<PFN_GetPhysicalDeviceProcAddr>,
}

#[repr(C)]
pub struct VkLayerInstanceCreateInfo {
    pub s_type: vk::StructureType,
    pub p_next: *const c_void,
    pub function: VkLayerFunction,
    /* This should properly be represented as union with PFN_vkSetInstanceLoaderData,
       In practice, it doesn't matter.
     */
    pub p_layer_info: *const VkLayerInstanceLink,
}
#[repr(C)]
pub struct VkLayerInstanceLink {
    pub p_next: *const VkLayerInstanceLink,
    pub pfn_next_get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
    pub pfn_next_get_device_proc_addr: vk::PFN_vkGetDeviceProcAddr,
}

#[repr(C)]
pub struct VkLayerDeviceCreateInfo {
    pub s_type: vk::StructureType,
    pub p_next: *const c_void,
    pub function: VkLayerFunction,
    pub p_layer_info: *const VkLayerDeviceLink,
}

#[repr(C)]
pub struct VkLayerDeviceLink {
    pub p_next: *const VkLayerDeviceLink,
    pub pfn_next_get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
    pub pfn_next_get_device_proc_addr: vk::PFN_vkGetDeviceProcAddr,
}

pub struct InstanceDispatchTable {
    pub get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
    pub destroy_instance: vk::PFN_vkDestroyInstance,
}

pub struct DeviceDispatchTable {
    pub get_device_proc_addr: vk::PFN_vkGetDeviceProcAddr,
    pub destroy_device: vk::PFN_vkDestroyDevice,
}

static LOAD_CONFIG: SyncOnceCell<LoadConfig> = SyncOnceCell::new();
static FIRST_INSTANCE: SyncOnceCell<vk::Instance> = SyncOnceCell::new();
static FIRST_DEVICE: SyncOnceCell<vk::Device> = SyncOnceCell::new();

#[allow(clippy::type_complexity)]
static mut INSTANCE: SyncLazy<
    RwLock<HashMap<vk::Instance, InstanceDispatchTable>>,
> = SyncLazy::new(Default::default);

#[allow(clippy::type_complexity)]
static mut DEVICE: SyncLazy<
    RwLock<HashMap<vk::Device, DeviceDispatchTable>>,
> = SyncLazy::new(Default::default);

unsafe extern "system" fn create_instance(
    p_create_info: *const vk::InstanceCreateInfo,
    p_allocator: *const vk::AllocationCallbacks,
    p_instance: *mut vk::Instance,
) -> vk::Result {
    println!("[sk] create_instance");
    let instance_info = p_create_info.as_ref().unwrap();

    let mut layer_info = instance_info.p_next.cast::<VkLayerInstanceCreateInfo>();
    while !layer_info.is_null() &&
            ((*layer_info).s_type != vk::StructureType::LOADER_INSTANCE_CREATE_INFO
                || (*layer_info).function != VkLayerFunction::VK_LAYER_FUNCTION_LINK)
    {
        // I have no idea if this is safe lol
        layer_info = (*layer_info).p_next.cast::<VkLayerInstanceCreateInfo>()
    }

    if layer_info.is_null() {
        return Result::ERROR_INITIALIZATION_FAILED;
    }

    // Don't move link
    let next_layer_info = (*layer_info).p_layer_info.read();

    // move chain on for next layer
    // this is so bad.
    (*layer_info.as_mut()).p_layer_info = next_layer_info.p_next;

    let gpa = next_layer_info.pfn_next_get_instance_proc_addr;

    // hippity hoppity your PFN_vkVoidFunction is now a PFN_vkCreateInstance
    let fp_create_instance: vk::PFN_vkCreateInstance = std::mem::transmute(gpa(vk::Instance::null(), b"vkCreateInstance\0".as_ptr() as *const c_char));

    let result = fp_create_instance(p_create_info, p_allocator, p_instance);

    let dispatch = InstanceDispatchTable {
        get_instance_proc_addr: gpa,
        destroy_instance: std::mem::transmute(gpa(*p_instance, b"vkDestroyInstance\0".as_ptr() as *const c_char))
    };

    let (result, boot_clr) = (move || {
        INSTANCE.write().ok()?.insert(*p_instance, dispatch);
        let is_first = FIRST_INSTANCE.get().is_none();
        FIRST_INSTANCE.get_or_init(|| *p_instance);
        let clr_opt = LOAD_CONFIG.get()?.vulkan()?.entry;
        Some((result, Some(clr_opt == VulkanEntryPoint::CreateInstance && is_first)))
    })().unwrap_or((Result::ERROR_INITIALIZATION_FAILED, None));

    // At this point we are not allowed to fail.

    if let Some(true) = boot_clr {
        // Boot CLR here in separate thread.
        clr_entry_point(vec![p_instance.read().as_raw()]);
    }

    return result;
}

unsafe extern "system" fn destroy_instance(
    instance: vk::Instance,
    p_allocator: *const vk::AllocationCallbacks,
) {
    (|| {
        let dispatch = INSTANCE.write().ok()?.remove(&instance);
        if let Some(dispatch) = dispatch {
            (dispatch.destroy_instance)(instance, p_allocator);
        }
        Some(())
    })().unwrap_or(())
}

// https://android.googlesource.com/platform/cts/+/6743db1/hostsidetests/gputools/layers/jni/nullLayer.cpp
unsafe extern "system" fn create_device(
    physical_device: vk::PhysicalDevice,
    p_create_info: *const vk::DeviceCreateInfo,
    p_allocator: *const vk::AllocationCallbacks,
    p_device: *mut vk::Device,
) -> vk::Result {
    println!("[sk] create_device");

    let instance_info = p_create_info.as_ref().unwrap();

    let mut layer_info = instance_info.p_next.cast::<VkLayerDeviceCreateInfo>();
    while !layer_info.is_null() &&
        ((*layer_info).s_type != vk::StructureType::LOADER_DEVICE_CREATE_INFO
            || (*layer_info).function != VkLayerFunction::VK_LAYER_FUNCTION_LINK)
    {
        // I have no idea if this is safe lol
        layer_info = (*layer_info).p_next.cast::<VkLayerDeviceCreateInfo>()
    }

    if layer_info.is_null() {
        return Result::ERROR_INITIALIZATION_FAILED;
    }

    // Don't move link
    let next_layer_info = (*layer_info).p_layer_info.read();

    let gipa = next_layer_info.pfn_next_get_instance_proc_addr;
    let gdpa = next_layer_info.pfn_next_get_device_proc_addr;

    // move chain on for next layer
    // this is so bad.
    (*layer_info.as_mut()).p_layer_info = next_layer_info.p_next;

    let fp_create_device: vk::PFN_vkCreateDevice = std::mem::transmute(gipa(vk::Instance::null(), b"vkCreateDevice\0".as_ptr() as *const c_char));
    let result = fp_create_device(physical_device, p_create_info, p_allocator, p_device);

    let dispatch = DeviceDispatchTable {
        get_device_proc_addr: gdpa,
        destroy_device: std::mem::transmute(gdpa(*p_device, b"vkDestroyDevice\0".as_ptr() as *const c_char))
    };

    let (result, boot_clr, instance) = (move || {
        DEVICE.write().ok()?.insert(*p_device, dispatch);
        let is_first = FIRST_DEVICE.get().is_none();
        FIRST_DEVICE.get_or_init(|| *p_device);
        let clr_opt = LOAD_CONFIG.get()?.vulkan()?.entry;
        Some((result, Some(clr_opt == VulkanEntryPoint::CreateDevice && is_first), FIRST_INSTANCE.get()))
    })().unwrap_or((Result::ERROR_INITIALIZATION_FAILED, None, None));

    // At this point we are not allowed to fail.

    if let Some(true) = boot_clr {
        // Boot CLR here in separate thread.
        if let Some(instance) = instance {
            clr_entry_point(vec![instance.as_raw(), p_device.read().as_raw()]);
        }
    }

    return result;
}

unsafe extern "system" fn destroy_device(
    device: vk::Device,
    p_allocator: *const vk::AllocationCallbacks,
) {
    (|| {
        let dispatch = DEVICE.write().ok()?.remove(&device);
        if let Some(dispatch) = dispatch {
            (dispatch.destroy_device)(device, p_allocator);
        }
        Some(())
    })().unwrap_or(())
}

#[no_mangle]
pub unsafe extern "system" fn get_device_proc_addr(
    device: vk::Device,
    p_name: *const std::os::raw::c_char,
) -> vk::PFN_vkVoidFunction {
    let name = CStr::from_ptr(p_name);
    match name.to_bytes() {
        b"vkGetDeviceProcAddr" => Some(std::mem::transmute(get_device_proc_addr as vk::PFN_vkGetDeviceProcAddr)),
        b"vkCreateDevice" => Some(std::mem::transmute(create_device as vk::PFN_vkCreateDevice)),
        b"vkDestroyDevice" => Some(std::mem::transmute(destroy_device as vk::PFN_vkDestroyDevice)),
        _ => DEVICE.read().ok()?.get(&device)
            .map(|dispatch| (dispatch.get_device_proc_addr)(device, p_name))
            .unwrap_or(None)
    }
}

#[no_mangle]
pub unsafe extern "system" fn get_instance_proc_addr(
    instance: vk::Instance,
    p_name: *const std::os::raw::c_char,
) -> vk::PFN_vkVoidFunction {
    let name = CStr::from_ptr(p_name);
    match name.to_bytes() {
        b"vkGetInstanceProcAddr" => Some(std::mem::transmute(get_instance_proc_addr as vk::PFN_vkGetInstanceProcAddr)),
        b"vkCreateInstance" => Some(std::mem::transmute(create_instance as vk::PFN_vkCreateInstance)),
        b"vkDestroyInstance" => Some(std::mem::transmute(destroy_instance as vk::PFN_vkDestroyInstance)),

        b"vkGetDeviceProcAddr" => Some(std::mem::transmute(get_device_proc_addr as vk::PFN_vkGetDeviceProcAddr)),
        b"vkCreateDevice" => Some(std::mem::transmute(create_device as vk::PFN_vkCreateDevice)),
        b"vkDestroyDevice" => Some(std::mem::transmute(destroy_device as vk::PFN_vkDestroyDevice)),
        _ => INSTANCE.read().ok()?.get(&instance)
            .map(|dispatch| (dispatch.get_instance_proc_addr)(instance, p_name))
            .unwrap_or(None)
    }
}

fn clr_entry_point(handles: Vec<u64>) {
    thread::spawn(move || {
        let config = match LOAD_CONFIG.get() {
            Some(config) => config,
            None => {
                eprintln!("[vk_inject] Error occurred when injecting CLR");
                return 1
            }
        };

        match crate::boot_clr(config.clone(),Some(handles)) {
            Ok(i) => {
                i as u32
            }
            Err(e) => {
                eprintln!("[vk_inject] Error occurred when injecting CLR: {}", e);
                1 as u32
            }
        }
    });
}

#[no_mangle]
pub unsafe extern "system" fn sabinokaku_negotiate_layer_version(interface: *mut VkNegotiateLayerInterface) -> vk::Result {
    // unsafe { winapi::um::consoleapi::AllocConsole(); }

    if (*interface).s_type != VkLayerNegotiateStructType::LAYER_NEGOTIATE_INTERFACE_STRUCT {
        return Result::ERROR_INITIALIZATION_FAILED;
    }

    if let Some(true) = std::env::var_os("SABINOKAKU_VULKAN_BOOTED").map(|s| s == OsStr::new("1")) {
        eprintln!("[dllmain_inject] Vulkan env already initialized.");
        return Result::ERROR_INITIALIZATION_FAILED;
    }

    // Do not allow reinitialization.
    if FIRST_INSTANCE.get().is_some() {
        return Result::ERROR_INITIALIZATION_FAILED;
    }

    let target_ld = (*interface).loader_layer_interface_version;

    if target_ld < 2 {
        // We only support Layer Interface Version 2.
        return Result::ERROR_INITIALIZATION_FAILED;
    }

    if let Ok(config) = crate::get_config() {
        // Validate init params
        let vk_cfg = config.vulkan();
        if vk_cfg.is_none() {
            return Result::ERROR_INITIALIZATION_FAILED
        }

        let vk_cfg = vk_cfg.unwrap();

        if target_ld >= vk_cfg.loader_version {
            (*interface).loader_layer_interface_version = vk_cfg.loader_version;
            (*interface).pfn_get_device_proc_addr = get_device_proc_addr;
            (*interface).pfn_get_instance_proc_addr = get_instance_proc_addr;
        }

        (*interface).pfn_get_physical_device_proc_addr = None;

        LOAD_CONFIG.get_or_init(move || config);
        println!("Negotiate OK {:?}\n IPA {:p}\n {:p}", LOAD_CONFIG.get(), (*interface).pfn_get_instance_proc_addr as *const (),  (*interface).pfn_get_device_proc_addr as *const ());
        return Result::SUCCESS
    }
    println!("failed vulkan");
    Result::ERROR_INITIALIZATION_FAILED
}