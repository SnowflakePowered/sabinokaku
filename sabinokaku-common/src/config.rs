use std::env::current_exe;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::{FromStr, Lines};

use netcorehost::pdcstring::PdCString;
use crate::config::AdditionalParameter::{DotNetRoot, EnvironmentVariable, Hostfxr, Vulkan};

#[derive(Debug, Clone)]
pub struct LoadConfig {
    pub runtime_config: PdCString,
    pub type_name: PdCString,
    pub entry_method: PdCString,
    pub entry_assembly: PdCString,
    pub additional_params: Vec<AdditionalParameter>,
}

#[derive(Debug)]
pub enum ConfigError {
    MissingOrInvalidConfigMagic(Option<String>),
    InvalidConfig,
    MissingConfig
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VulkanEntryPoint {
    CreateDevice,
    CreateInstance
}

#[derive(Debug, Clone)]
pub enum AdditionalParameter {
    EnvironmentVariable(OsString, OsString),
    Hostfxr(PathBuf),
    DotNetRoot(PathBuf),
    Vulkan(VulkanInitParams)
}

#[derive(Debug, Clone)]
pub struct VulkanInitParams {
    pub loader_version: u32,
    pub entry: VulkanEntryPoint
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
               additional_params: Vec<AdditionalParameter>) -> LoadConfig {
        LoadConfig { runtime_config, type_name, entry_method, entry_assembly, additional_params }
    }

    pub fn try_parse(root: PathBuf, input: &dyn AsRef<str>) -> Result<LoadConfig, Box<dyn Error>> {
        let mut input = input.as_ref();

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

    pub fn environment_variables(&self) -> impl Iterator<Item=(&OsStr, &OsStr)> {
        self.additional_params.iter().filter_map(|p| match p {
            AdditionalParameter::EnvironmentVariable(k, v) => {
                Some((k.as_os_str(), v.as_os_str()))
            }
            _ => None
        })
    }

    pub fn hostfxr_path(&self) -> Option<&Path> {
        self.additional_params.iter()
            .find_map(|f| match f {
                AdditionalParameter::Hostfxr(p) => Some(p.as_path()),
                _ => None
            })
    }

    pub fn dotnetroot_path(&self) -> Option<&Path> {
        self.additional_params.iter()
            .find_map(|f| match f {
                AdditionalParameter::DotNetRoot(p) => Some(p.as_path()),
                _ => None
            })
    }

    pub fn vulkan(&self) -> Option<&VulkanInitParams> {
        self.additional_params.iter()
            .find_map(|f| match f {
                AdditionalParameter::Vulkan(vep) => Some(vep),
                _ => None
            })
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

        let additional = Self::parse_additional(&root, &lines[4..]);

        Ok(LoadConfig::new(
            PdCString::from_os_str(runtime_config_path.as_os_str())?,
            PdCString::from_os_str(assembly_fname_path.as_os_str())?,
            PdCString::from_str(entry_type)?,
            PdCString::from_str(entry_fn)?,
            additional
        ))
    }

    fn parse_short(root: PathBuf, mut input: Lines) -> Result<LoadConfig, Box<dyn Error>> {
        let line = input.next().ok_or(ConfigError::InvalidConfig)?;
        let (asm, rest) = line.split_once("::").ok_or(ConfigError::InvalidConfig)?;
        let (entry_cls, entry_fn) = rest.split_once("!").ok_or(ConfigError::InvalidConfig)?;

        let root = root.parent().ok_or(ConfigError::MissingConfig)?;

        let mut runtime_config_path = PathBuf::from(root);
        runtime_config_path.push(&format!("{}.runtimeconfig.json", asm));

        let mut assembly_fname_path = PathBuf::from(root);
        assembly_fname_path.push(&format!("{}.dll", asm));

        let lines: Vec<&str> = input.collect();
        let additional = Self::parse_additional(&root, &lines);
        Ok(LoadConfig::new(
            PdCString::from_os_str(runtime_config_path.as_os_str())?,
            PdCString::from_os_str(assembly_fname_path.as_os_str())?,
            PdCString::from_str(&format!("{}, {}", entry_cls, asm))?,
            PdCString::from_str(entry_fn)?,
            additional
        ))
    }

    fn parse_additional(root: &Path, input: &[&str]) -> Vec<AdditionalParameter> {
        let mut map = Vec::new();

        for line in input {
            match line.split_once(" ") {
                Some(("env", env)) => {
                    if let Some((k, v)) = env.split_once("=") {
                        map.push(EnvironmentVariable(OsString::from(k), OsString::from(v)));
                    }
                }
                Some(("hostfxr", hostfxr)) => {
                    let mut buf = PathBuf::from(root);
                    buf.push(hostfxr);
                    map.push(Hostfxr(buf));
                }
                Some(("dotnetroot", dotnetroot)) => {
                    let mut buf = PathBuf::from(root);
                    buf.push(dotnetroot);
                    map.push(DotNetRoot(buf));
                }
                Some(("vulkan", vulkan)) => {
                    if let Some((ld, entry)) = vulkan.split_once(" ") {
                        if let Ok(ld) = ld.parse() {
                            let entry_point = match entry {
                                "CreateInstance" | "vkCreateInstance" => VulkanEntryPoint::CreateInstance,
                                "CreateDevice" | "vkCreateDevice" => VulkanEntryPoint::CreateDevice,
                                _ => continue
                            };
                            map.push(Vulkan(VulkanInitParams { loader_version: ld, entry: entry_point }))
                        }
                    }
                }
                _ => {}
            }
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::str::FromStr;
    use netcorehost::pdcstr;
    use crate::LoadConfig;

    #[test]
    fn test_parse_short() {
        let kaku_co = "kaku_s
Assembly::TestInject.EntryPoint!Main";
        let config = LoadConfig::try_parse(PathBuf::from("kaku.co"), &kaku_co).unwrap();
        assert_eq!(config.runtime_config.as_ref(), pdcstr!("Assembly.runtimeconfig.json"));
        assert_eq!(config.type_name.as_ref(), pdcstr!("TestInject.EntryPoint, Assembly"));
        assert_eq!(config.entry_assembly.as_ref(), pdcstr!("Assembly.dll"));
        assert_eq!(config.entry_method.as_ref(), pdcstr!("Main"));
        assert_eq!(config.additional_params.len(), 0);
    }

    #[test]
    fn test_parse_short_params() {
        let kaku_co = "kaku_s
Assembly::TestInject.EntryPoint!Main
hostfxr HOSTFX
env TESTENV=TEST
env TESTENV2=TEST2
dotnetroot DOTNETROOT
";
        let config = LoadConfig::try_parse(PathBuf::from("kaku.co"), &kaku_co).unwrap();
        assert_eq!(config.runtime_config.as_ref(), pdcstr!("Assembly.runtimeconfig.json"));
        assert_eq!(config.type_name.as_ref(), pdcstr!("TestInject.EntryPoint, Assembly"));
        assert_eq!(config.entry_assembly.as_ref(), pdcstr!("Assembly.dll"));
        assert_eq!(config.entry_method.as_ref(), pdcstr!("Main"));
        assert_eq!(config.environment_variables().collect::<Vec<_>>(), vec![
            (OsString::from_str("TESTENV").unwrap().as_os_str(), OsString::from_str("TEST").unwrap().as_os_str()),
            (OsString::from_str("TESTENV2").unwrap().as_os_str(), OsString::from_str("TEST2").unwrap().as_os_str())
        ]);
        assert_eq!(config.hostfxr_path(), Some(Path::new("HOSTFX")));
        assert_eq!(config.dotnetroot_path(), Some(Path::new("DOTNETROOT")));
    }

    #[test]
    fn test_parse_long() {
        let kaku_co = "kaku_l
Assembly.runtimeconfig.json
Assembly.dll
TestInject.EntryPoint, Assembly
Main";
        let config = LoadConfig::try_parse(PathBuf::from("kaku.co"), &kaku_co).unwrap();
        assert_eq!(config.runtime_config.as_ref(), pdcstr!("Assembly.runtimeconfig.json"));
        assert_eq!(config.type_name.as_ref(), pdcstr!("TestInject.EntryPoint, Assembly"));
        assert_eq!(config.entry_assembly.as_ref(), pdcstr!("Assembly.dll"));
        assert_eq!(config.entry_method.as_ref(), pdcstr!("Main"));
        assert_eq!(config.additional_params.len(), 0);
    }

    #[test]
    fn test_parse_long_params() {
        let kaku_co = "kaku_l
Assembly.runtimeconfig.json
Assembly.dll
TestInject.EntryPoint, Assembly
Main
hostfxr HOSTFX
env TESTENV=TEST
env TESTENV2=TEST2
dotnetroot DOTNETROOT
";
        let config = LoadConfig::try_parse(PathBuf::from("/kaku.co"), &kaku_co).unwrap();
        assert_eq!(config.runtime_config.as_ref(), pdcstr!("/Assembly.runtimeconfig.json"));
        assert_eq!(config.type_name.as_ref(), pdcstr!("TestInject.EntryPoint, Assembly"));
        assert_eq!(config.entry_assembly.as_ref(), pdcstr!("/Assembly.dll"));
        assert_eq!(config.entry_method.as_ref(), pdcstr!("Main"));
        assert_eq!(config.environment_variables().collect::<Vec<_>>(), vec![
            (OsString::from_str("TESTENV").unwrap().as_os_str(), OsString::from_str("TEST").unwrap().as_os_str()),
            (OsString::from_str("TESTENV2").unwrap().as_os_str(), OsString::from_str("TEST2").unwrap().as_os_str())
        ]);
        assert_eq!(config.hostfxr_path(), Some(Path::new("/HOSTFX")));
        assert_eq!(config.dotnetroot_path(), Some(Path::new("/DOTNETROOT")));
    }
}
