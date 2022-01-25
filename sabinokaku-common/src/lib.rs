use std::error::Error;

use netcorehost::nethost;

use crate::config::LoadConfig;

pub mod config;

pub mod prelude {
    pub use crate::config::*;
    pub use crate::init_clr;
}

pub fn init_clr(config: LoadConfig) -> Result<i32, Box<dyn Error>> {
    for (key, value) in config.env_vars {
        std::env::set_var(key, value);
    }
    let hostfxr = nethost::load_hostfxr()?;
    let context = hostfxr.initialize_for_runtime_config(&config.runtime_config)?;
    let loader = context.get_delegate_loader_for_assembly(&config.entry_assembly)?;
    let init = loader.get_function_pointer_with_default_signature(config.type_name, config.entry_method)?;
    Ok(unsafe { init(std::ptr::null(), 0) })
}