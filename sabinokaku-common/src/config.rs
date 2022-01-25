use std::env::current_exe;
use std::error::Error;
use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::{FromStr, Lines};

use netcorehost::pdcstring::PdCString;

#[derive(Debug)]
pub struct LoadConfig {
    pub runtime_config: PdCString,
    pub type_name: PdCString,
    pub entry_method: PdCString,
    pub entry_assembly: PdCString,
    pub env_vars: Vec<(OsString, OsString)>,
}

#[derive(Debug)]
pub enum ConfigError {
    MissingOrInvalidConfigMagic(Option<String>),
    InvalidConfig,
    MissingConfig
}

pub trait ConfigSearchPath {
    fn get_module_path() -> Option<PathBuf>;

    fn search_for_config() -> Result<PathBuf, Box<dyn Error>> {
        let module_parent = Self::get_module_path();
        if let Some(Some(mut kaku_path)) = module_parent.map(|s| s.parent().map(PathBuf::from)) {
            kaku_path.push("kaku.co");
            if kaku_path.exists() {
                return Ok(kaku_path);
            }
        }

        if let Ok(Some(mut kaku_path)) = current_exe()
            .map(|s| s.parent().map(PathBuf::from)) {
            kaku_path.push("kaku.co");
            if kaku_path.exists() {
                return Ok(kaku_path);
            }
        }

        Err(Box::new(ConfigError::MissingConfig))
    }
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingOrInvalidConfigMagic(s) => write!(f, "Configuration file magic number is missing, should be kaku_l or kaku_s, was {:?}.", s),
            ConfigError::InvalidConfig => write!(f, "Configuration file is malformed."),
            ConfigError::MissingConfig => write!(f, "kaku.co configuration file not found."),
        }
    }
}

impl Error for ConfigError {}

impl LoadConfig {
    pub fn new(runtime_config: PdCString, entry_assembly: PdCString, type_name: PdCString, entry_method: PdCString,
               env_vars: Vec<(OsString, OsString)>) -> LoadConfig {
        LoadConfig { runtime_config, type_name, entry_method, entry_assembly, env_vars }
    }

    pub fn try_parse(root: PathBuf, input: String) -> Result<LoadConfig, Box<dyn Error>> {
        let mut input = input.as_str();

        // deal with BOM.
        if input.starts_with("\u{feff}") {
            input = &input.trim_start_matches("\u{feff}");
        }

        let mut lines = input.lines();
        match lines.next() {
            Some("kaku_s") => LoadConfig::parse_short(root, lines),
            Some("kaku_l") => LoadConfig::parse_long(root, lines),
            x => Err(Box::new(ConfigError::MissingOrInvalidConfigMagic(x.map(String::from))))
        }
    }

    fn parse_long(root: PathBuf, input: Lines) -> Result<LoadConfig, Box<dyn Error>> {
        let lines: Vec<&str> = input.collect();
        if lines.len() < 4 {
            return Err(Box::new(ConfigError::InvalidConfig));
        }
        let runtime_config = lines[0];
        let assembly_fname = lines[1];
        let entry_type = lines[2];
        let entry_fn = lines[3];

        let root = root.parent().ok_or(ConfigError::MissingConfig)?;

        let mut runtime_config_path = PathBuf::from(root);
        runtime_config_path.push(runtime_config);

        let mut assembly_fname_path = PathBuf::from(root);
        assembly_fname_path.push(assembly_fname);
        let envvars = Self::parse_env(&lines[4..]);

        Ok(LoadConfig::new(
            PdCString::from_os_str(runtime_config_path.as_os_str())?,
            PdCString::from_os_str(assembly_fname_path.as_os_str())?,
            PdCString::from_str(entry_type)?,
            PdCString::from_str(entry_fn)?,
            envvars
        ))
    }

    fn parse_short(root: PathBuf, mut input: Lines) -> Result<LoadConfig, Box<dyn Error>> {
        let line = input.next().ok_or(ConfigError::InvalidConfig)?;
        let (asm, rest) = line.split_once("::").ok_or(ConfigError::InvalidConfig)?;
        let (entry_cls, entry_fn) = rest.split_once("$").ok_or(ConfigError::InvalidConfig)?;

        let root = root.parent().ok_or(ConfigError::MissingConfig)?;

        let mut runtime_config_path = PathBuf::from(root);
        runtime_config_path.push(&format!("{}.runtimeconfig.json", asm));

        let mut assembly_fname_path = PathBuf::from(root);
        assembly_fname_path.push(&format!("{}.dll", asm));

        let lines: Vec<&str> = input.collect();
        let envvars = Self::parse_env(&lines);

        Ok(LoadConfig::new(
            PdCString::from_os_str(runtime_config_path.as_os_str())?,
            PdCString::from_os_str(assembly_fname_path.as_os_str())?,
            PdCString::from_str(&format!("{}, {}", entry_cls, asm))?,
            PdCString::from_str(entry_fn)?,
            envvars
        ))
    }

    fn parse_env(input: &[&str]) -> Vec<(OsString, OsString)> {
        let mut vars = Vec::new();

        for line in input {
            if !line.starts_with("env ") { continue; }
            let line = &line["env ".len()..];
            if let Some((k, v)) = line.split_once("=") {
                vars.push((OsString::from(k), OsString::from(v)))
            }
        }
        vars
    }
}