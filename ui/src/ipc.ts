import { invoke } from "@tauri-apps/api/core";

/** Version of the running app, from the Rust shell. Rejects when not running inside Tauri. */
export function appVersion(): Promise<string> {
  return invoke<string>("app_version");
}
