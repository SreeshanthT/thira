import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  Group,
  Panel,
  Separator,
  useDefaultLayout,
  type LayoutStorage,
} from "react-resizable-panels";
import { Viewer } from "./Viewer";

/**
 * Keeps panel sizes in the webview's local storage. Reading or writing can
 * throw (for example when storage is disabled); then the default layout is used
 * and nothing is saved.
 */
export const layoutStorage: LayoutStorage = {
  getItem(key) {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  },
  setItem(key, value) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // Not persisted; the layout still works for this session.
    }
  },
};

/** Storage key prefix for a layout; bump the version when panels change. */
const LAYOUT_ID = "thira.layout.v1";

function useSavedLayout({ id }: { id: string }) {
  return useDefaultLayout({
    id: `${LAYOUT_ID}.${id}`,
    storage: layoutStorage,
    // Window resizes rescale panels but must not overwrite what the user chose.
    onlySaveAfterUserInteractions: true,
  });
}

/** A titled panel region with placeholder content until its feature lands. */
function Pane({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="pane" aria-label={title}>
      <h2 className="pane-title">{title}</h2>
      <div className="pane-body">{children}</div>
    </section>
  );
}

/**
 * The editing workspace (PRD APP-2): media pool, viewer and inspector side by
 * side above the timeline. Every boundary can be dragged or moved with the
 * arrow keys, and sizes persist between sessions.
 */
export function Workspace() {
  const { t } = useTranslation();
  const rows = useSavedLayout({ id: "rows" });
  const columns = useSavedLayout({ id: "columns" });

  return (
    <Group
      className="workspace"
      orientation="vertical"
      defaultLayout={rows.defaultLayout}
      onLayoutChanged={rows.onLayoutChanged}
    >
      <Panel id="top" defaultSize="60%" minSize="25%">
        <Group
          orientation="horizontal"
          defaultLayout={columns.defaultLayout}
          onLayoutChanged={columns.onLayoutChanged}
        >
          <Panel id="media" defaultSize="20%" minSize="10%">
            <Pane title={t("panels.media.title")}>
              <p className="placeholder">{t("panels.media.empty")}</p>
            </Pane>
          </Panel>
          <Separator className="separator" aria-label={t("panels.resize")} />
          <Panel id="viewer" defaultSize="60%" minSize="25%">
            <section className="viewer-pane" aria-label={t("panels.viewer.title")}>
              <Viewer />
            </section>
          </Panel>
          <Separator className="separator" aria-label={t("panels.resize")} />
          <Panel id="inspector" defaultSize="20%" minSize="10%">
            <Pane title={t("panels.inspector.title")}>
              <p className="placeholder">{t("panels.inspector.empty")}</p>
            </Pane>
          </Panel>
        </Group>
      </Panel>
      <Separator className="separator" aria-label={t("panels.resize")} />
      <Panel id="timeline" defaultSize="40%" minSize="15%">
        <Pane title={t("panels.timeline.title")}>
          <p className="placeholder">{t("panels.timeline.empty")}</p>
        </Pane>
      </Panel>
    </Group>
  );
}
