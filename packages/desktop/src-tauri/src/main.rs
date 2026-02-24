// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use tauri::{api::path::app_cache_dir, api::path::app_config_dir, Manager};
use std::env;
use std::path::PathBuf;

mod downloader;
mod rmbg;

use std::process::Command;

#[tauri::command]
fn open_program(path: String, program: String) {
    let mut final_program_path = program.clone();

    // Se for o Figma, vamos procurar a pasta dele dinamicamente!
    if program == "Figma.exe" {
        // Pega o caminho C:\Users\SEU_USUARIO\AppData\Local
        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            let figma_dir = PathBuf::from(local_app_data).join("Figma");
            
            // Lê as pastas dentro de LocalAppData\Figma
            if let Ok(entries) = fs::read_dir(figma_dir) {
                for entry in entries.flatten() {
                    let dir_path = entry.path();
                    // Procura a pasta que começa com "app-" (ex: app-126.1.2)
                    if dir_path.is_dir() && dir_path.file_name().unwrap().to_str().unwrap().starts_with("app-") {
                        let exe_path = dir_path.join("Figma.exe");
                        if exe_path.exists() {
                            final_program_path = exe_path.to_str().unwrap().to_string();
                            break;
                        }
                    }
                }
            }
        }
    }

    println!("Caminho final do executável: {}", final_program_path);

    // Executa o programa passando o caminho da imagem
    let _ = Command::new(&final_program_path)
        .arg(&path)
        .spawn();
}

#[tauri::command]
async fn load_config(app_handle: tauri::AppHandle) -> Result<String, ()> {
    let app_config = app_handle.config();
    let config_dir = app_config_dir(&app_config).ok_or(())?;
    let config_path = config_dir.join("config.json");
    let mut config_file = fs::File::open(config_path).or(Err(()))?;
    let mut config = String::new();
    config_file.read_to_string(&mut config).or(Err(()))?;
    Ok(config)
}

#[tauri::command]
async fn save_config(config: String, app_handle: tauri::AppHandle) -> Result<(), ()> {
    let app_config = app_handle.config();
    let config_dir = app_config_dir(&app_config).ok_or(())?;
    fs::create_dir_all(&config_dir).or(Err(()))?;

    let config_path = config_dir.join("config.json");
    let mut file = fs::File::create(config_path).or(Err(()))?;
    file.write_all(config.as_bytes()).or(Err(()))?;
    Ok(())
}

#[tauri::command]
async fn download_model(
    name: String,
    version: String,
    url: String,
    app_handle: tauri::AppHandle,
) -> Result<String, ()> {
    let app_config = app_handle.config();
    let cache_dir = app_cache_dir(&app_config).ok_or(())?;
    fs::create_dir_all(&cache_dir).or(Err(()))?;

    let output_path = Path::new(&cache_dir).join(format!("{}-{}.onnx", name, version));
    let output = output_path.to_str().ok_or(())?;
    downloader::download(url, output.to_string(), move |progress| {
        app_handle
            .emit_all(&format!("model/download/progress/{}", &name), progress)
            .unwrap();
    })
    .await?;
    Ok(output.to_string())
}

#[tauri::command]
async fn rmbg(file: String, model: String, resolution: u32) -> Result<String, String> {
    match rmbg::process_image(file.as_str(), model.as_str(), resolution) {
        Ok(image) => Ok(image),
        Err(e) => {
            eprintln!("RMBG Error: {:?}", e);
            eprintln!("  file: {}", file);
            eprintln!("  model: {}", model);
            eprintln!("  resolution: {}", resolution);
            Err(format!("{:?}", e))
        },
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            open_program,
            load_config,
            save_config,
            download_model,
            rmbg
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
