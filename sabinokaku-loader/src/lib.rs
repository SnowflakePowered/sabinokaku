#![crate_type = "cdylib"]
#![feature(once_cell)]
#![feature(ptr_const_cast)]

#[cfg(all(target_os = "windows"))]
mod windows;

#[cfg(all(target_os = "linux"))]
mod linux;

#[cfg(feature = "vulkan")]
mod vulkan;

use std::error::Error;
use std::io::Read;
use sabinokaku_common::prelude::*;

#[cfg(all(target_os = "windows"))]
use crate::windows::WindowsConfigSearchPath as OsConfigSearchPath;

#[cfg(all(target_os = "linux"))]
use crate::linux::LinuxConfigSearchPath as OsConfigSearchPath;

fn get_config() -> Result<LoadConfig, Box<dyn Error>> {
    let cfg_path = OsConfigSearchPath::search_for_config()?;
    let mut file = std::fs::File::open(&cfg_path)?;
    let mut cfg_string = String::new();
    file.read_to_string(&mut cfg_string)?;
    LoadConfig::try_parse(cfg_path, &cfg_string)
}

fn boot_clr<T>(config: LoadConfig, args: Option<Vec<T>>) -> Result<i32, Box<dyn Error>> {
    // println!("booting clr {:?}", config);
    Ok(sabinokaku_common::init_clr(config, args)?)
}
