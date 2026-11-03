import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { appVersion, isEngineError, projectInfo, type ProjectInfo } from "./ipc";
import * as project from "./project";
import { Viewer } from "./Viewer";

// Placeholder shell; the panel layout arrives in #15.
export function App() {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string | null>(null);
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    appVersion().then(setVersion, () => setVersion(null));
    projectInfo().then(setInfo, () => setInfo(null));
  }, []);

  /** Runs a project action, updating the shown project and reporting failures. */
  const run = useCallback(async (action: () => Promise<ProjectInfo | null | undefined>) => {
    setError(null);
    try {
      const result = await action();
      if (result !== undefined) setInfo(result);
    } catch (err) {
      setError(isEngineError(err) ? err.message : String(err));
    }
  }, []);

  const onNew = useCallback(() => run(project.newProject), [run]);
  const onOpen = useCallback(() => run(project.openProject), [run]);
  const onSaveAs = useCallback(() => run(project.saveProjectAs), [run]);
  const onSave = useCallback(
    () => run(async () => (info ? project.saveProject(info) : undefined)),
    [run, info],
  );
  const onClose = useCallback(
    () => run(async () => ((await project.closeProject()) ? null : undefined)),
    [run],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      const shortcuts: Record<string, () => Promise<void>> = {
        n: onNew,
        o: onOpen,
        s: e.shiftKey ? onSaveAs : onSave,
      };
      const action = shortcuts[e.key.toLowerCase()];
      if (action) {
        e.preventDefault();
        void action();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onNew, onOpen, onSave, onSaveAs]);

  // Ask before the window closes with unsaved changes.
  useEffect(() => {
    // Registration is async: if the effect is cleaned up first (StrictMode remounts),
    // unregister as soon as it completes so only one prompt ever appears.
    let unlisten: (() => void) | undefined;
    let cleanedUp = false;
    getCurrentWindow()
      .onCloseRequested(async (event) => {
        if (!(await project.confirmQuit())) event.preventDefault();
      })
      .then(
        (fn) => (cleanedUp ? fn() : (unlisten = fn)),
        () => undefined,
      );
    return () => {
      cleanedUp = true;
      unlisten?.();
    };
  }, []);

  return (
    <main className="app">
      <header className="toolbar">
        <button onClick={onNew}>{t("toolbar.new")}</button>
        <button onClick={onOpen}>{t("toolbar.open")}</button>
        <button onClick={onSave} disabled={!info}>
          {t("toolbar.save")}
        </button>
        <button onClick={onSaveAs} disabled={!info}>
          {t("toolbar.saveAs")}
        </button>
        <button onClick={onClose} disabled={!info}>
          {t("toolbar.close")}
        </button>
        <span className="project" aria-live="polite">
          {info ? (
            <>
              <strong>{info.name}</strong>
              {info.dirty && <span title={t("project.unsavedMark")}> •</span>}
              <span className="path">{info.path ?? t("project.notSaved")}</span>
            </>
          ) : (
            t("project.none")
          )}
        </span>
        {version && <span className="version">{t("app.version", { version })}</span>}
      </header>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <section className="workspace">
        <Viewer />
      </section>
    </main>
  );
}
