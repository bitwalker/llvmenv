use dirs;
use failure::{bail, err_msg};
use log::info;
use std::fs;
use std::env;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use crate::error::Result;

pub const APP_NAME: &'static str = "llvmenv";
pub const ENTRY_TOML: &'static str = "entry.toml";

const LLVM_MIRROR: &str = include_str!("llvm-mirror.toml");

fn is_absolute_path(path: OsString) -> Option<PathBuf> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Some(path)
    } else {
        None
    }
}

// We do this here and on the other functions below, because 
// for some reason the 'dirs' library does not follow the XDG 
// specification on macOS
#[cfg(target_os = "macos")]
pub fn config_dir() -> Result<PathBuf> {
    let path = env::var_os("XDG_CONFIG_HOME")
        .and_then(is_absolute_path)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .ok_or(err_msg("Unsupported OS"))?
        .join(APP_NAME);
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    Ok(path)
}

#[cfg(not(target_os = "macos"))]
pub fn config_dir() -> Result<PathBuf> {
    let path = dirs::config_dir()
        .ok_or(err_msg("Unsupported OS"))?
        .join(APP_NAME);
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    Ok(path)
}

#[cfg(target_os = "macos")]
pub fn cache_dir() -> Result<PathBuf> {
    let path = env::var_os("XDG_CACHE_HOME")
        .and_then(is_absolute_path)
        .or_else(|| dirs::home_dir().map(|h| h.join(".cache")))
        .ok_or(err_msg("Unsupported OS"))?
        .join(APP_NAME);
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    Ok(path)
}

#[cfg(not(target_os = "macos"))]
pub fn cache_dir() -> Result<PathBuf> {
    let path = dirs::cache_dir()
        .ok_or(err_msg("Unsupported OS"))?
        .join(APP_NAME);
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    Ok(path)
}

#[cfg(target_os = "macos")]
pub fn data_dir() -> Result<PathBuf> {
    let path = env::var_os("XDG_DATA_HOME")
        .and_then(is_absolute_path)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
        .ok_or(err_msg("Unsupported OS"))?
        .join(APP_NAME);
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    Ok(path)
}

#[cfg(not(target_os = "macos"))]
pub fn data_dir() -> Result<PathBuf> {
    let path = dirs::data_dir()
        .ok_or(err_msg("Unsupported OS"))?
        .join(APP_NAME);
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    Ok(path)
}

/// Initialize configure file
pub fn init_config() -> Result<()> {
    let dir = config_dir()?;
    let entry = dir.join(ENTRY_TOML);
    if !entry.exists() {
        info!("Create default entry setting: {}", entry.display());
        let mut f = fs::File::create(entry)?;
        f.write(LLVM_MIRROR.as_bytes())?;
    } else {
        bail!("Setting already exists.");
    }
    Ok(())
}
