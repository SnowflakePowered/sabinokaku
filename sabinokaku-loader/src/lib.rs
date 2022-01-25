#![crate_type = "cdylib"]
#![feature(once_cell)]

#[cfg(all(target_os = "windows"))]
mod windows;

#[cfg(all(target_os = "linux"))]
mod linux;

use std::error::Error;
use std::io::Read;
use sabinokaku_common::prelude::*;

#[cfg(all(target_os = "windows"))]
use crate::windows::WindowsConfigSearchPath as OsConfigSearchPath;

#[cfg(all(target_os = "linux"))]
use crate::linux::LinuxConfigSearchPath as OsConfigSearchPath;

fn main() -> Result<i32, Box<dyn Error>> {
    let cfg_path = OsConfigSearchPath::search_for_config()?;
    let mut file = std::fs::File::open(&cfg_path)?;
    let mut cfg_string = String::new();
    file.read_to_string(&mut cfg_string)?;
    let config = LoadConfig::try_parse(cfg_path, cfg_string)?;
    Ok(sabinokaku_common::init_clr(config)?)
}

