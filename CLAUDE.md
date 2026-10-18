# jman

Desktop app + CLI that switches the active JDK without restarting terminals or IDEs.

## Core idea (don't break it)
`JAVA_HOME` and `PATH` are set **once** to `~/.jman/current` (+ `~/.jman/current/bin`, `~/.jman/bin`).
`current` is a link (NTFS junction on Windows, symlink on Unix). Switching = re-pointing that link, nothing else.
Never switch by editing environment variables, and never use `setx` (truncates PATH at 1024 chars, expands `%VAR%`).

## Layout
- `crates/jman-core` — all logic, no Tauri dependency. Sync code only (`ureq`, no tokio).
  - `detect` JDK discovery (reads each JDK's `release` file) · `link` junction/symlink switching · `select` query → JDK (`21`, `1.8`, `temurin-21`)
  - `setup/windows.rs` registry (raw `REG_EXPAND_SZ`, `WM_SETTINGCHANGE`, UAC via `ShellExecuteExW runas`) · `setup/unix.rs` shell-profile block between `# >>> jman >>>` markers
  - `download` foojay Disco API v3, SHA-256 check, extracts into `~/.jman/jdks` · `doctor` diagnostics
  - `remove` `plan()` picks how to remove a JDK (delete folder / Windows uninstaller / apt·dnf·brew·scoop), `execute()` runs it. Elevated deletes on Windows re-launch the app with `--elevated-remove <path>` (CLI: `remove --elevated-path`), which re-checks the path is a JDK
- `crates/jman-cli` — binary named `jman` (clap).
- `src-tauri` — package `jman-app` (not `jman`, to avoid colliding with the CLI binary in `target/`). Commands in `lib.rs` are thin wrappers that run core code through `blocking()`; tray in `tray.rs`.
- `src` — SvelteKit in SPA mode (adapter-static), Svelte 5 runes, TypeScript. `src/lib/api.ts` holds typed `invoke` wrappers; Rust structs serialize as camelCase.

## Commands
```
cargo test --workspace --exclude jman-app   # core + CLI tests
cargo clippy --workspace                    # keep it warning-free
npm run check                               # svelte-check
npm run tauri dev                           # builds the CLI in release first, then runs the app
npm run tauri build
```

## Testing safely
- Set `JMAN_HOME` to a scratch folder so the link, config and downloads don't touch the real `~/.jman`. To test removal, copy a JDK into a scratch folder listed in that sandbox's `config.json` `extraScanDirs`.
- The installed app may be running in the tray; the dev build has the same single-instance id, so quit it first.
- **Never run `jman setup` / the app's "Set up", or remove a real JDK, without asking.** It writes the user's real system environment (HKLM registry on Windows, shell profiles on Unix), and `JMAN_HOME` doesn't sandbox that part.
- macOS/Linux paths are only unit-tested and built in CI; flag Unix-specific changes as unverified unless tested on a real machine.

## Non-obvious wiring
- The CLI ships inside the app as a resource (`cli/jman[.exe]`), configured per OS in `tauri.{windows,macos,linux}.conf.json` from `target/release/`. `beforeDevCommand`/`beforeBuildCommand` build it first; `cli_source()` falls back to a sibling binary in dev.
- Windows elevation: the app relaunches itself with `--elevated-setup` (handled in `src-tauri/src/main.rs` before Tauri starts). The CLI uses `jman setup --system-only`. The user-level part (HKCU `JAVA_HOME`) runs unelevated, because the elevated process may run as a different account.
- The UI and tray refresh on the `jdks-changed` event. A background thread polls the link every 2s so CLI switches show up in the GUI.
- Closing the window hides it to the tray; single-instance plugin focuses the existing window.
- New plugin APIs used from the frontend need permissions in `src-tauri/capabilities/default.json` (kept minimal).
- Download progress is throttled to ~1 MB steps in core; keep it that way, since the CLI and the Tauri `Channel` both consume it.

## Conventions
- Commit subjects: short and lowercase (`jman cli`, `tray: rebuild menu after switch`).
- Code comments explain *why*, sparingly. Match the existing style.
