use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::{paths, presets, Profile};

pub const DEFAULT_PROFILE: &str = "default";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Name of the profile currently on screen.
    pub active: String,
    /// Connector name of the output to draw on (`DP-5`, `eDP-1`, ...).
    /// `None` means "first monitor GDK reports".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor: Option<String>,
    /// Whether the overlay is shown; persisted so a restart keeps your state.
    pub visible: bool,
    pub profiles: BTreeMap<String, Profile>,
}

impl Default for Config {
    fn default() -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert(DEFAULT_PROFILE.to_owned(), Profile::default());
        for (name, profile) in presets::all() {
            profiles.insert(name.to_owned(), profile);
        }
        Self {
            active: DEFAULT_PROFILE.to_owned(),
            monitor: None,
            visible: true,
            profiles,
        }
    }
}

impl Config {
    /// Loads the config, writing a commented default file when none exists.
    pub fn load_or_create() -> Result<Self> {
        let path = paths::config_file();
        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut config: Self =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        if config.profiles.is_empty() {
            config
                .profiles
                .insert(DEFAULT_PROFILE.to_owned(), Profile::default());
        }
        if !config.profiles.contains_key(&config.active) {
            // Don't fail on a typo — fall back to something drawable.
            config.active = config
                .profiles
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| DEFAULT_PROFILE.to_owned());
        }
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = paths::config_file();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let body = toml::to_string_pretty(self).context("serialising config")?;
        // Write-then-rename so a reload never sees a half-written file.
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, body).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))?;
        Ok(())
    }

    pub fn active_profile(&self) -> Profile {
        self.profiles.get(&self.active).copied().unwrap_or_default()
    }

    pub fn profile_names(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }

    /// Name of the profile after `active`, wrapping around.
    pub fn next_profile(&self) -> Option<String> {
        let names = self.profile_names();
        let idx = names.iter().position(|n| *n == self.active)?;
        names.get((idx + 1) % names.len()).cloned()
    }
}

/// True when the config file is a symlink into the Nix store, i.e. it is
/// managed declaratively (home-manager and friends).
///
/// Such a file must never be written: the rename in [`Config::save`] would
/// replace the symlink with a regular file and quietly detach the config from
/// the generation that owns it.
pub fn is_managed() -> bool {
    let path = paths::config_file();
    let Ok(meta) = std::fs::symlink_metadata(&path) else {
        return false;
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    std::fs::canonicalize(&path)
        .map(|target| target.starts_with("/nix/store"))
        .unwrap_or(false)
}
