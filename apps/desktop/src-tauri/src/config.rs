use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

const CONFIG_FILE: &str = "pet-element.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct PetConfig {
  pub focus_minutes: u32,
  pub break_minutes: u32,
  pub water_interval_minutes: u32,
  pub stand_interval_minutes: u32,
  pub pet_x: f64,
  pub pet_y: f64,
  pub panel_open: bool,
  pub phase: String,
  pub remaining_ms: u64,
  pub active_focus_length_ms: u64,
  pub focus_sessions: u32,
  pub last_water_at: u64,
  pub last_stand_at: u64,
}

impl Default for PetConfig {
  fn default() -> Self {
    let now = timestamp_ms();
    Self {
      focus_minutes: 25,
      break_minutes: 5,
      water_interval_minutes: 45,
      stand_interval_minutes: 60,
      pet_x: 300.0,
      pet_y: 620.0,
      panel_open: false,
      phase: "idle".to_string(),
      remaining_ms: 25 * 60 * 1000,
      active_focus_length_ms: 25 * 60 * 1000,
      focus_sessions: 0,
      last_water_at: now,
      last_stand_at: now,
    }
  }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
  pub config: PetConfig,
  pub platform: String,
}

fn timestamp_ms() -> u64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|duration| duration.as_millis() as u64)
    .unwrap_or(0)
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
  let dir = app.path().app_config_dir().map_err(|error| error.to_string())?;
  fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
  Ok(dir.join(CONFIG_FILE))
}

pub fn load_from_disk(app: &AppHandle) -> Result<PetConfig, String> {
  let path = config_path(app)?;
  if !path.exists() {
    return Ok(PetConfig::default());
  }

  let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
  serde_json::from_str::<PetConfig>(&content).map_err(|error| error.to_string())
}

pub fn save_to_disk(app: &AppHandle, config: &PetConfig) -> Result<(), String> {
  let path = config_path(app)?;
  let content = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
  fs::write(path, content).map_err(|error| error.to_string())
}
