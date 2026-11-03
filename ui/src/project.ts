import { ask, open, save } from "@tauri-apps/plugin-dialog";
import i18n from "./i18n";
import * as ipc from "./ipc";
import type { ProjectInfo } from "./ipc";

const EXTENSION = "thira";

function filters() {
  return [{ name: i18n.t("project.fileType"), extensions: [EXTENSION] }];
}

/**
 * Runs an action that replaces the open project. If the engine refuses because of
 * unsaved changes, asks the user and retries with `force` when they agree.
 * Resolves to `undefined` if the user cancels.
 */
async function replacing<T>(action: (force: boolean) => Promise<T>): Promise<T | undefined> {
  try {
    return await action(false);
  } catch (err) {
    if (!ipc.isEngineError(err) || err.code !== "unsaved") throw err;
    const discard = await ask(i18n.t("dialog.discardPrompt"), {
      title: i18n.t("dialog.unsavedTitle"),
      kind: "warning",
      okLabel: i18n.t("dialog.discard"),
      cancelLabel: i18n.t("dialog.cancel"),
    });
    return discard ? action(true) : undefined;
  }
}

export function newProject(): Promise<ProjectInfo | undefined> {
  return replacing((force) => ipc.newProject(force));
}

export async function openProject(): Promise<ProjectInfo | undefined> {
  const path = await open({ filters: filters(), multiple: false, directory: false });
  if (path === null) return undefined;
  return replacing((force) => ipc.openProject(path, force));
}

export async function saveProjectAs(): Promise<ProjectInfo | undefined> {
  const path = await save({
    filters: filters(),
    defaultPath: `${i18n.t("project.untitled")}.${EXTENSION}`,
  });
  if (path === null) return undefined;
  return ipc.saveProjectAs(path);
}

/** Saves to the current file, or asks for one if the project was never saved. */
export function saveProject(current: ProjectInfo): Promise<ProjectInfo | undefined> {
  return current.path === null ? saveProjectAs() : ipc.saveProject();
}

/** Resolves to `true` once the project is closed, `false` if the user cancelled. */
export async function closeProject(): Promise<boolean> {
  const done = await replacing(async (force) => {
    await ipc.closeProject(force);
    return true;
  });
  return done === true;
}

/** Whether it is fine to quit: nothing unsaved, or the user agreed to discard it. */
export async function confirmQuit(): Promise<boolean> {
  const info = await ipc.projectInfo();
  if (!info?.dirty) return true;
  return ask(i18n.t("dialog.quitPrompt"), {
    title: i18n.t("dialog.unsavedTitle"),
    kind: "warning",
    okLabel: i18n.t("dialog.quit"),
    cancelLabel: i18n.t("dialog.cancel"),
  });
}
