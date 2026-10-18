use std::io::Write;
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};
use jman_core::download::{self, Progress};
use jman_core::{detect, doctor, paths, remove, select, setup};

/// Switch Java versions instantly, without restarting terminals or IDEs.
#[derive(Parser)]
#[command(name = "jman", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List installed JDKs
    #[command(alias = "ls")]
    List,
    /// Show the selected JDK
    Current,
    /// Switch to a JDK: an id, a version (21, 1.8, 17.0.9) or vendor[-version] (temurin-21)
    Use { query: String },
    /// Point JAVA_HOME and PATH at jman (once)
    Setup {
        /// Only write the system variables (used internally when elevating on Windows)
        #[arg(long, hide = true)]
        system_only: bool,
    },
    /// Download and install a JDK, e.g. `temurin@21`, `zulu@17` or `21`
    Install {
        spec: String,
        /// Switch to it once installed
        #[arg(long = "use")]
        use_it: bool,
    },
    /// List the distributions and versions available for download
    Available,
    /// Remove a JDK: deletes jman's downloads, runs the vendor uninstaller or package manager for others
    #[command(alias = "rm")]
    Remove {
        #[arg(required_unless_present = "elevated_path")]
        query: Option<String>,
        /// Don't ask for confirmation
        #[arg(long, short)]
        yes: bool,
        /// Delete this JDK folder (used internally when elevating on Windows)
        #[arg(long, hide = true)]
        elevated_path: Option<std::path::PathBuf>,
    },
    /// Diagnose the setup
    Doctor,
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::List => {
            let current = jman_core::current().map(|j| detect::canonical_key(&j.home));
            let jdks = detect::detect_all();
            if jdks.is_empty() {
                println!("No JDKs found. Install one with `jman install temurin@21`.");
            }
            let width = jdks.iter().map(|j| j.id.len()).max().unwrap_or(0);
            for jdk in jdks {
                let marker = if Some(detect::canonical_key(&jdk.home)) == current { "*" } else { " " };
                println!(
                    "{marker} {:width$}  {:12}  {:16}  {}",
                    jdk.id,
                    jdk.version,
                    jdk.source,
                    jdk.home.display()
                );
            }
        }
        Command::Current => match jman_core::current() {
            Some(jdk) => println!("{} ({}) -> {}", jdk.id, jdk.version, jdk.home.display()),
            None => println!("No JDK selected. Run `jman use <version>`."),
        },
        Command::Use { query } => {
            let jdk = jman_core::use_jdk(&query)?;
            println!("Now using {} ({})", jdk.id, jdk.version);
        }
        Command::Setup { system_only } => setup_cmd(system_only)?,
        Command::Install { spec, use_it } => {
            let (distro, major) = download::parse_spec(&spec)?;
            let pkg = download::find_package(&distro, major)?
                .with_context(|| format!("no {distro} {major} build for this platform"))?;
            println!("Installing {} {}...", pkg.distribution, pkg.java_version);
            let jdk = download::install(&pkg, print_progress)?;
            println!("\r{:<50}\rInstalled {} at {}", "", jdk.id, jdk.home.display());
            if use_it {
                jman_core::link::switch(&jdk.home)?;
                println!("Now using {}", jdk.id);
            }
        }
        Command::Available => {
            let versions = download::major_versions()?;
            let list: Vec<String> = versions
                .iter()
                .map(|v| format!("{}{}", v.major_version, if v.term_of_support.eq_ignore_ascii_case("lts") { " (LTS)" } else { "" }))
                .collect();
            println!("Versions: {}", list.join(", "));
            println!("Distributions:");
            for d in download::DISTRIBUTIONS {
                println!("  {:20} {}", d.id, d.name);
            }
            println!("\nExample: jman install temurin@{}", versions.first().map(|v| v.major_version).unwrap_or(21));
        }
        Command::Remove { elevated_path: Some(path), .. } => remove::delete_elevated(&path)?,
        Command::Remove { query, yes, .. } => {
            let jdks = detect::detect_all();
            let jdk = select::resolve(query.as_deref().unwrap_or_default(), &jdks)?;
            let removal = remove::plan(jdk)?;
            println!("{} ({}): {}", jdk.id, jdk.source, remove::describe(&removal));
            if !jdk.managed && !yes && !confirm("Proceed?")? {
                return Ok(());
            }
            remove::execute(jdk, &removal, elevate_delete)?;
            println!("Removed {}", jdk.id);
        }
        Command::Doctor => {
            let report = doctor::run();
            println!("jman dir:  {}", report.jman_dir.display());
            match &report.current {
                Some(jdk) => println!("selected:  {} ({})", jdk.id, jdk.version),
                None => println!("selected:  none"),
            }
            match &report.java_on_path {
                Some(java) => println!("java:      {}", java.display()),
                None => println!("java:      not on PATH"),
            }
            let problems: Vec<&String> = report.setup.issues.iter().chain(&report.warnings).collect();
            if problems.is_empty() {
                println!("\nEverything looks good.");
            } else {
                println!();
                for p in problems {
                    println!("! {p}");
                }
                if !report.setup.configured {
                    println!("\nRun `jman setup` to fix the setup.");
                }
            }
        }
    }
    Ok(())
}

fn confirm(question: &str) -> anyhow::Result<bool> {
    print!("{question} [y/N] ");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

/// Deletes a JDK folder from an elevated copy of this binary (Windows).
fn elevate_delete(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe()?;
        let code = setup::windows::run_elevated(&exe, &format!("remove --elevated-path \"{}\"", path.display()))?;
        anyhow::ensure!(code == 0, "elevated delete failed with exit code {code}");
        Ok(())
    }
    #[cfg(not(windows))]
    anyhow::bail!("cannot elevate to delete {}", path.display())
}

fn setup_cmd(system_only: bool) -> anyhow::Result<()> {
    #[cfg(windows)]
    if system_only {
        return setup::windows::apply_system();
    }
    #[cfg(not(windows))]
    let _ = system_only;

    let exe = std::env::current_exe()?;
    setup::install_cli(&exe)?;
    setup::apply_user()?;

    #[cfg(windows)]
    if setup::status().needs_elevation {
        if setup::windows::can_write_system() {
            setup::windows::apply_system()?;
        } else {
            println!("Asking for administrator rights to set the system JAVA_HOME and PATH...");
            let installed = paths::bin_dir().join(paths::exe_name("jman"));
            let code = setup::windows::run_elevated(&installed, "setup --system-only")?;
            anyhow::ensure!(code == 0, "elevated setup failed with exit code {code}");
        }
    }

    let status = setup::status();
    if let Some(jdk) = jman_core::current() {
        println!("Selected JDK: {} ({})", jdk.id, jdk.version);
    }
    for issue in &status.issues {
        println!("! {issue}");
    }
    println!("JAVA_HOME -> {}", paths::current_link().display());
    println!("Done. Open a new terminal (and restart your IDE once); after that, switching is instant.");
    Ok(())
}

fn print_progress(p: Progress) {
    let line = match p {
        Progress::Downloading { done, total } if total > 0 => {
            format!("Downloading {:>3}% ({} / {} MB)", done * 100 / total, done >> 20, total >> 20)
        }
        Progress::Downloading { done, .. } => format!("Downloading {} MB", done >> 20),
        Progress::Verifying => "Verifying checksum...".into(),
        Progress::Extracting => "Extracting...".into(),
    };
    print!("\r{line:<50}");
    let _ = std::io::stdout().flush();
}
