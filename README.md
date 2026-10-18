<div align="center">

# jman
<img src="jman.png" width=20%>

![](https://img.shields.io/github/downloads/DxxxxY/jman/total?style=for-the-badge)

![](https://img.shields.io/badge/Rust-black?style=for-the-badge&logo=rust&logoColor=#E57324)
![](https://img.shields.io/badge/Svelte-FF3E00?style=for-the-badge&logo=Svelte&logoColor=white)
![](https://img.shields.io/badge/Tauri-0388fc?style=for-the-badge&logo=Tauri&logoColor=white)

Switch Java versions instantly, **without restarting your terminals or IDEs**.

</div>

## How it works
Other version switchers rewrite `JAVA_HOME` and `PATH`. Programs that are already running never see that change, so you have to restart every terminal and IDE.

jman sets the variables **once** and points them at one folder that never moves:

```
JAVA_HOME = ~/.jman/current
PATH      = ~/.jman/current/bin;~/.jman/bin;...
```

`~/.jman/current` is a link: a directory junction on Windows, a symlink on macOS and Linux. Switching JDKs only re-points that link. It takes milliseconds and needs no admin rights. Every open shell and IDE picks up the new JDK the next time it runs `java`.

## Features
- Instant switching from the app (click, or <kbd>↑</kbd>/<kbd>↓</kbd>), from the tray menu, or with `jman use 21`.
- Finds JDKs in the usual install folders: Program Files (Oracle, Temurin, Zulu, Corretto, Microsoft, Liberica…), `/Library/Java/JavaVirtualMachines`, `/usr/lib/jvm`, IntelliJ's `~/.jdks`, Gradle toolchains, SDKMAN and scoop. You can also add your own folders.
- Shows where each JDK came from, and removes any of them: it deletes jman's own downloads and plain folders, runs the vendor's uninstaller for installed JDKs (Windows "Installed apps"), and uses apt/dnf/Homebrew/scoop for package-managed ones, asking for admin rights only when needed.
- Downloads JDKs from Temurin, Zulu, Corretto, Liberica, GraalVM and more through the [foojay Disco API](https://api.foojay.io), and verifies their checksums.
- Windows, macOS and Linux.

## Setup
1. Install jman from the latest [release](https://github.com/DxxxxY/jman/releases).
2. Open it and click **Set up**:
   - **Windows:** sets the system `JAVA_HOME` and `PATH`. Windows shows one administrator prompt. Switching never needs admin afterwards.
   - **macOS / Linux:** adds a marked block to your shell profiles (`~/.zshrc`, `~/.bashrc`, …).
     - On macOS it also adds a LaunchAgent so GUI apps get `JAVA_HOME`.
     - On Linux it also writes `~/.config/environment.d/jman.conf`.
3. Open a new terminal and restart your IDE **one last time**. From then on, switching applies immediately.

### IDEs
Set your IDE's JDK to `~/.jman/current`, not to a specific JDK.
- IntelliJ caches the version it detected for an SDK. After switching to a different major version, reload the SDK in *Project Structure*.
- Gradle and Maven daemons keep the JDK they started with. Run `gradle --stop` after switching.

## CLI
Setup installs the `jman` command into `~/.jman/bin`:

```
jman list                 # installed JDKs, * marks the selected one
jman use 21               # by version: 21, 1.8, 17.0.9
jman use temurin-21       # by vendor/name and version
jman current
jman install zulu@17      # download a JDK (--use to switch to it)
jman available            # downloadable versions and distributions
jman remove temurin-21    # remove any JDK (asks first; --yes to skip)
jman doctor               # explain what is wrong with the setup
jman setup                # the same one-time setup as the app
```

## Development
Requires Node.js and Rust ([Tauri prerequisites](https://tauri.app/start/prerequisites/)).

```
npm install
npm run tauri dev       # builds the CLI, then runs the app
npm run tauri build     # installers in target/release/bundle
cargo test --workspace --exclude jman-app
```

Set `JMAN_HOME` to a scratch folder to experiment without touching your real `~/.jman`.

| Path | What |
| --- | --- |
| `crates/jman-core` | All logic: detection, switching, setup, downloads |
| `crates/jman-cli` | The `jman` command |
| `src-tauri` | The desktop app and tray |
| `src` | The Svelte 5 UI |

## Disclaimer
This is for educational purposes only. I am not responsible for any damage caused by this tool.

The v2 rewrite was made almost entirely with Claude Opus 5.5 on Medium/High effort.

## License
GPLv3 © dxxxxy
