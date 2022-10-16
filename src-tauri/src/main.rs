#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

//imports
use std::fs;
use std::process::Command;
use is_elevated::is_elevated;
use tauri::{Window, Manager};

#[tauri::command]
fn get_installed_jdks() -> Vec<String> {
    //initialize vector
    let mut jdks = Vec::new();

    //for each directory in the java folder that starts with jdk
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

    //return vector
    jdks
}

#[tauri::command]
fn set_jdk(jdk: &str, window: tauri::Window) {
    log(&window, &format!("Setting JDK to {}", jdk));

    //get array of paths from PATH
    log(&window, "Getting paths from PATH");
    let paths = std::env::var("PATH").unwrap();
    let mut paths = paths.split(";").collect::<Vec<&str>>();
    
    //delete old jdks and "javapath"s from paths
    log(&window, "Deleting old JDKs and javapaths from PATH");
    paths.retain(|&p| !p.contains("jdk") && !p.contains("javapath"));

    //recreate cleaned PATH string
    let mut paths = paths.join(";");

    //add the new jdk to the PATH
    log(&window, "Adding new JDK to PATH");
    paths.push_str(&format!(";C:\\Program Files\\Java\\{}\\bin", jdk));

    //set PATH (windows only)
    log(&window, "Setting PATH");
    let command = Command::new("cmd")
        .args(&["/C", "setx", "PATH", "/M", &paths])
        .output()
        .expect("Failed to set PATH");

    let stdout = String::from_utf8_lossy(&command.stdout);
    let stderr = String::from_utf8_lossy(&command.stderr);

    if stderr.is_empty() {
        log(&window, &stdout.into_owned());
    } else { 
        log(&window, &stderr.into_owned());
    }

    //set JAVA_HOME (windows only)
    log(&window, "Setting JAVA_HOME");
    Command::new("cmd")
        .args(&["/C", "setx", "JAVA_HOME", "/M", &format!("C:\\Program Files\\Java\\{}", jdk)])
        .output()
        .expect("Failed to set JAVA_HOME");

    let stdout = String::from_utf8_lossy(&command.stdout);
    let stderr = String::from_utf8_lossy(&command.stderr);

    if stderr.is_empty() {
        log(&window, &stdout.into_owned());
    } else { 
        log(&window, &stderr.into_owned());
    }
}

#[tauri::command]
fn get_current_jdk() -> String {
    //get the current JAVA version in use
    let command = match Command::new("java")
        .arg("-version")
        .output() { //in case java is not installed (or not setup properly)
            Ok(output) => output,
            Err(_) => return "".to_string()
        };
    
    //get the outputs
    let stdout = String::from_utf8_lossy(&command.stdout);
    let stderr = String::from_utf8_lossy(&command.stderr);

    //on some versions like java 8, the version appears in stderr
    //on others like java 9, the version appears in stdout
    if stderr.is_empty() {
        return stdout.into_owned();
    } else { 
        return stderr.into_owned();
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            if !is_elevated() {
                app.get_window("main").unwrap().eval("window.isElevated = false").unwrap();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_installed_jdks, get_current_jdk, set_jdk])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn log(window: &Window, message: &str) {
    Window::emit(&window, "log", message).unwrap();
}