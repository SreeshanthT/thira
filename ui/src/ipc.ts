import { invoke } from "@tauri-apps/api/core";

/** Mirrors `thira_engine::ProjectInfo`. */
export interface ProjectInfo {
  name: string;
  path: string | null;
  dirty: boolean;
  version: number;
  undo_label: string | null;
  redo_label: string | null;
}

/** Mirrors the serialized `thira_engine::EngineError`. */
export interface EngineError {
  code: string;
  message: string;
}

export function isEngineError(value: unknown): value is EngineError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}

/** Version of the running app, from the Rust shell. Rejects when not running inside Tauri. */
export function appVersion(): Promise<string> {
  return invoke<string>("app_version");
}

export function newProject(force: boolean): Promise<ProjectInfo> {
  return invoke("new_project", { force });
}

export function openProject(path: string, force: boolean): Promise<ProjectInfo> {
  return invoke("open_project", { path, force });
}

export function saveProject(): Promise<ProjectInfo> {
  return invoke("save_project");
}

export function saveProjectAs(path: string): Promise<ProjectInfo> {
  return invoke("save_project_as", { path });
}

export function closeProject(force: boolean): Promise<void> {
  return invoke("close_project", { force });
}

export function projectInfo(): Promise<ProjectInfo | null> {
  return invoke("project_info");
}
