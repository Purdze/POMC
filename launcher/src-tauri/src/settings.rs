use crate::storage;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    pub language: String,
    pub keep_launcher_open: bool,
    pub launch_with_console: bool,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        LauncherSettings {
            language: "English".into(),
            keep_launcher_open: true,
            launch_with_console: false,
        }
    }
}

impl LauncherSettings {
    pub fn save(&self) -> Result<(), String> {
        let path = storage::settings_file();
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load() -> Self {
        let path = storage::settings_file();

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<LauncherSettings>(&content) {
                Ok(cfg) => return cfg,
                Err(err) => {
                    log::warn!("Settings file invalid ({}), using defaults", err);
                }
            },
            Err(_) => {
                log::info!("Settings file not found, creating default settings");
            }
        }

        let default = LauncherSettings::default();
        let _ = default.save();
        default
    }

    pub fn update_settings<F>(f: F) -> Result<(), String>
    where
        F: FnOnce(&mut LauncherSettings),
    {
        let mut settings = LauncherSettings::load();
        f(&mut settings);
        settings.save()
    }
}
