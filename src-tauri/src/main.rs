#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::fs;
use std::process::Command;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_installed_jdks() -> Vec<String> {
    let mut jdks = Vec::new();
    for entry in fs::read_dir("C:\\Program Files\\Java").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_str().unwrap();
            if dir_name.starts_with("jdk") {
                jdks.push(dir_name.to_string());
            }
        }
    }
    jdks
}

#[tauri::command]
fn get_current_jdk() -> String {
    let command = Command::new("java")
        .arg("-version")
        .output()
        .expect("Failed to execute java -version command!");
    
    let stdout = String::from_utf8_lossy(&command.stdout);
    let stderr = String::from_utf8_lossy(&command.stderr);
    
    if stderr.is_empty() {
        return stdout.into_owned();
    } else { 
        return stderr.into_owned();
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_installed_jdks, greet, get_current_jdk])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
