#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::fs;
use std::process::Command;
// use is_elevated::is_elevated;

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
fn set_jdk(jdk: &str) -> String {
    println!("Setting JDK to {}", jdk);

    //get array of paths from PATH
    let paths = std::env::var("PATH").unwrap();
    let mut paths = paths.split(";").collect::<Vec<&str>>();
    //print paths

    paths.retain(|&p| !p.contains("jdk") && !p.contains("javapath"));

    for path in &paths {
        println!("{}", path);
    }

    let mut paths = paths.join(";");

    paths.push_str(&format!("C:\\Program Files\\Java\\{}\\bin", jdk));

    //set PATH
    let command = Command::new("cmd")
        .args(&["/C", "setx", "PATH", "/M", &paths])
        .output()
        .expect("Failed to set PATH");

    //print results
    let stdout = String::from_utf8_lossy(&command.stdout);
    let stderr = String::from_utf8_lossy(&command.stderr);
    
    if stderr.is_empty() {
        return stdout.into_owned();
    } else { 
        return stderr.into_owned();
    }

    //set JAVA_HOME
    // let command = Command::new("cmd")
    //     .args(&["/C", "setx", "JAVA_HOME", "/M", &format!("%PATH%;C:\\Program Files\\Java\\{}", jdk)])
    //     .output()
    //     .expect("Failed to set JAVA_HOME");

    // //print results
    // println!("stdout: {}", String::from_utf8_lossy(&command.stdout));
    // println!("stderr: {}", String::from_utf8_lossy(&command.stderr));

    // set_env::append("PATH", &format!("C:\\Program Files\\Java\\{}", jdk)).expect("Couldn't find PATH");
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
//   if !is_elevated() {
//     println!(
//         "Warning: the program isn’t running as elevated; some functionality may not work."
//     );
//   }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_installed_jdks, greet, get_current_jdk, set_jdk])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
