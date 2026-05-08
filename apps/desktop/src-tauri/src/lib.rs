mod config;

use config::{load_from_disk, save_to_disk, AppBootstrap, PetConfig};
use tauri::{LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

const PET_HALF_W: f64 = 72.0;
const PET_HALF_H: f64 = 78.0;
const PANEL_W: f64 = 286.0;
const PANEL_H: f64 = 400.0;
const PANEL_GAP: f64 = 8.0;
const PANEL_RAISE: f64 = 72.0;
const PANEL_EDGE_MARGIN: f64 = 24.0;

#[tauri::command]
fn load_bootstrap(app: tauri::AppHandle) -> Result<AppBootstrap, String> {
  let config = load_from_disk(&app).unwrap_or_default();
  Ok(AppBootstrap {
    config,
    platform: std::env::consts::OS.to_string(),
  })
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: PetConfig) -> Result<(), String> {
  save_to_disk(&app, &config)
}

#[tauri::command]
fn set_panel_visible(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
  let main = app
    .get_webview_window("main")
    .ok_or_else(|| "main window not found".to_string())?;

  let mut config = load_from_disk(&app).unwrap_or_default();
  config.panel_open = visible;
  save_to_disk(&app, &config)?;

  if visible {
    let pet_x = config.pet_x;
    let pet_y = config.pet_y;

    let (panel_x, panel_y) = match main.primary_monitor().map_err(|e| e.to_string())? {
      Some(monitor) => {
        let sf = monitor.scale_factor();
        let wa = monitor.work_area();
        let wa_left = wa.position.x as f64 / sf;
        let wa_top = wa.position.y as f64 / sf;
        let wa_right = wa_left + wa.size.width as f64 / sf;
        let wa_bottom = wa_top + wa.size.height as f64 / sf;

        // Try right of pet
        let mut px = pet_x + PET_HALF_W + PANEL_GAP;
        if px + PANEL_W > wa_right {
          // Try left of pet
          px = pet_x - PET_HALF_W - PANEL_GAP - PANEL_W;
          if px < wa_left {
            px = wa_left;
          }
        }

        // Keep the panel a little higher than the pet anchor.
        let mut py = pet_y - PET_HALF_H - PANEL_RAISE;
        let min_y = wa_top + PANEL_EDGE_MARGIN;
        let max_y = (wa_bottom - PANEL_H - PANEL_EDGE_MARGIN).max(min_y);
        py = py.clamp(min_y, max_y);

        (px, py)
      }
      None => (
        pet_x + PET_HALF_W + PANEL_GAP,
        (pet_y - PET_HALF_H - PANEL_RAISE).clamp(PANEL_EDGE_MARGIN, f64::MAX)
      ),
    };

    main
      .set_position(LogicalPosition::new(panel_x, panel_y))
      .map_err(|e| e.to_string())?;
    main
      .set_size(LogicalSize::new(PANEL_W, PANEL_H))
      .map_err(|e| e.to_string())?;
    main.show().map_err(|e| e.to_string())?;
    main
      .set_ignore_cursor_events(false)
      .map_err(|e| e.to_string())?;
    main.set_focus().map_err(|e| e.to_string())?;
  } else {
    main.hide().map_err(|e| e.to_string())?;
    main
      .set_ignore_cursor_events(true)
      .map_err(|e| e.to_string())?;

    if let Some(pet) = app.get_webview_window("pet") {
      pet.set_focus().map_err(|e| e.to_string())?;
    }
  }

  Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![load_bootstrap, save_config, set_panel_visible])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      let handle = app.handle().clone();
      let config = load_from_disk(&handle).unwrap_or_default();

      if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|error| error.to_string())?;
        window
          .set_ignore_cursor_events(true)
          .map_err(|error| error.to_string())?;
      }

      let pet_window = WebviewWindowBuilder::new(
        &handle,
        "pet",
        WebviewUrl::App("index.html?view=pet".into()),
      )
      .title("PetElement")
      .transparent(true)
      .decorations(false)
      .always_on_top(true)
      .skip_taskbar(true)
      .shadow(false)
      .resizable(false)
      .inner_size(144.0, 156.0)
      .build()
      .map_err(|error| error.to_string())?;

      if let Some(monitor) = pet_window.primary_monitor().map_err(|error| error.to_string())? {
        let origin = monitor.work_area().position;
        let size = monitor.work_area().size;
        let scale_factor = monitor.scale_factor();
        let center_x = origin.x as f64 / scale_factor + (size.width as f64 / scale_factor) / 2.0;
        let center_y = origin.y as f64 / scale_factor + (size.height as f64 / scale_factor) / 2.0;
        let pet_x = if config.pet_x > 0.0 { config.pet_x } else { center_x };
        let pet_y = if config.pet_y > 0.0 { config.pet_y } else { center_y };
        pet_window
          .set_position(LogicalPosition::new(pet_x - PET_HALF_W, pet_y - PET_HALF_H))
          .map_err(|error| error.to_string())?;
      }

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
