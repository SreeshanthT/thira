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

/** Viewer backend chosen by the shell (ADR 0003). */
export type ViewerBackend = "native" | "transfer";

/** A rectangle in physical pixels from the top-left of the webview. */
export interface ViewerRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function viewerBackend(): Promise<ViewerBackend> {
  return invoke("viewer_backend");
}

/** Tells the native viewer where to draw; `null` hides it. */
export function setViewerRect(rect: ViewerRect | null): Promise<void> {
  return invoke("set_viewer_rect", { rect });
}

/** One RGBA frame from the transfer backend. */
export function viewerFrame(width: number, height: number): Promise<ArrayBuffer> {
  return invoke("viewer_frame", { width, height });
}
