use std::error::Error;
use netcorehost::hostfxr::Hostfxr;

use netcorehost::nethost;
use netcorehost::pdcstring::PdCString;

use crate::config::LoadConfig;

pub mod config;

pub mod prelude {
    pub use crate::config::*;
    pub use crate::init_clr;
}

pub fn init_clr(config: LoadConfig) -> Result<i32, Box<dyn Error>> {
    for (key, value) in config.environment_variables() {
        std::env::set_var(key, value);
    }

    let hostfxr = if let Some(hostfxr_path) = config.hostfxr_path() {
        Hostfxr::load_from_path(hostfxr_path)?
    } else {
        nethost::load_hostfxr()?
    };

    let context = if let Some(dotnet_path) = config.dotnetroot_path() {
        hostfxr.initialize_for_runtime_config_with_dotnet_root(&config.runtime_config,
                                                               PdCString::from_os_str(dotnet_path.as_os_str())?)?
    } else {
        hostfxr.initialize_for_runtime_config(&config.runtime_config)?
    };

    let loader = context.get_delegate_loader_for_assembly(&config.entry_assembly)?;
    let init = loader.get_function_pointer_with_default_signature(config.type_name, config.entry_method)?;
    Ok(unsafe { init(std::ptr::null(), 0) })
}