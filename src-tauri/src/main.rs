#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

//imports
use std::fs;
use std::process::Command;
use is_elevated::is_elevated;

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
fn set_jdk(jdk: &str) -> String {
    println!("Setting JDK to {}", jdk);

    //get array of paths from PATH
    let paths = std::env::var("PATH").unwrap();
    let mut paths = paths.split(";").collect::<Vec<&str>>();
    
    //delete old jdks and "javapath"s from paths
    paths.retain(|&p| !p.contains("jdk") && !p.contains("javapath"));

    //recreate cleaned PATH string
    let mut paths = paths.join(";");

    //add the new jdk to the PATH
    paths.push_str(&format!("C:\\Program Files\\Java\\{}\\bin", jdk));

    //set PATH (windows only)
    let command = Command::new("cmd")
        .args(&["/C", "setx", "PATH", "/M", &paths])
        .output()
        .expect("Failed to set PATH");

    //print results
    let stdout = String::from_utf8_lossy(&command.stdout);
    let stderr = String::from_utf8_lossy(&command.stderr);

    //set JAVA_HOME (windows only)
    let command = Command::new("cmd")
        .args(&["/C", "setx", "JAVA_HOME", "/M", &format!("C:\\Program Files\\Java\\{}", jdk)])
        .output()
        .expect("Failed to set JAVA_HOME");

    if stderr.is_empty() {
        return stdout.into_owned();
    } else { 
        return stderr.into_owned();
    }
}

#[tauri::command]
fn get_current_jdk() -> String {
    //get the current JAVA version in use
    let command = match Command::new("java")
        .arg("-version")
        .output() { //in case java is not installed (or not setup properly)
            Ok(output) => output,
            Err(_) => return "No JDK installed".to_string()
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
  if !is_elevated() {
      println!("Please run as administrator");
      return;
  }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_installed_jdks, get_current_jdk, set_jdk])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}