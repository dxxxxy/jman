import { Channel, invoke } from "@tauri-apps/api/core";

export interface Jdk {
  id: string;
  home: string;
  version: string;
  major: number;
  vendor: string | null;
  arch: string | null;
  managed: boolean;
  /** Where the JDK was found: "jman", "IntelliJ", "Eclipse Adoptium", "System", ... */
  source: string;
}

export type Removal =
  | { kind: "delete"; path: string; needsAdmin: boolean }
  | { kind: "uninstaller"; name: string; command: string }
  | { kind: "package"; manager: string; package: string; affected: string[] };

export interface SetupStatus {
  configured: boolean;
  issues: string[];
  needsElevation: boolean;
}

export interface DoctorReport {
  jmanDir: string;
  current: Jdk | null;
  setup: SetupStatus;
  javaOnPath: string | null;
  warnings: string[];
}

export interface Distribution {
  id: string;
  name: string;
}

export interface MajorVersion {
  majorVersion: number;
  termOfSupport: string;
}

export interface Package {
  id: string;
  distribution: string;
  majorVersion: number;
  javaVersion: string;
  archiveType: string;
  filename: string;
  size: number;
}

export type Progress =
  | { phase: "downloading"; done: number; total: number }
  | { phase: "verifying" }
  | { phase: "extracting" };

export interface Config {
  extraScanDirs: string[];
}

export const listJdks = () => invoke<Jdk[]>("list_jdks");
export const currentJdk = () => invoke<Jdk | null>("current_jdk");
export const useJdk = (id: string) => invoke<Jdk>("use_jdk", { id });
export const setupStatus = () => invoke<SetupStatus>("setup_status");
export const runSetup = () => invoke<SetupStatus>("run_setup");
export const doctor = () => invoke<DoctorReport>("doctor");
export const downloadOptions = () =>
  invoke<{ distributions: Distribution[]; versions: MajorVersion[] }>("download_options");
export const findPackage = (distribution: string, major: number) =>
  invoke<Package | null>("find_package", { distribution, major });
export const removalPlan = (id: string) => invoke<Removal>("removal_plan", { id });
export const removeJdk = (id: string) => invoke<void>("remove_jdk", { id });
export const getConfig = () => invoke<Config>("get_config");
export const setExtraScanDirs = (dirs: string[]) => invoke<void>("set_extra_scan_dirs", { dirs });

export function installJdk(distribution: string, major: number, onProgress: (p: Progress) => void) {
  const channel = new Channel<Progress>();
  channel.onmessage = onProgress;
  return invoke<Jdk>("install_jdk", { distribution, major, onProgress: channel });
}

export const formatMb = (bytes: number) => `${Math.round(bytes / 1048576)} MB`;
