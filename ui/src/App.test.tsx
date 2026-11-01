import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { ProjectInfo } from "./ipc";

const ipc = vi.hoisted(() => ({
  appVersion: vi.fn(),
  newProject: vi.fn(),
  openProject: vi.fn(),
  saveProject: vi.fn(),
  saveProjectAs: vi.fn(),
  closeProject: vi.fn(),
  projectInfo: vi.fn(),
  viewerBackend: vi.fn(),
  setViewerRect: vi.fn(),
  viewerFrame: vi.fn(),
}));
const dialog = vi.hoisted(() => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

vi.mock("./ipc", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ipc")>()),
  ...ipc,
}));
vi.mock("@tauri-apps/plugin-dialog", () => dialog);
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ onCloseRequested: () => Promise.resolve(() => undefined) }),
}));

function info(over: Partial<ProjectInfo> = {}): ProjectInfo {
  return {
    name: "Untitled",
    path: null,
    dirty: false,
    version: 0,
    undo_label: null,
    redo_label: null,
    ...over,
  };
}

async function click(name: string) {
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name }));
  });
}

beforeEach(() => {
  vi.resetAllMocks();
  ipc.appVersion.mockResolvedValue("1.2.3");
  ipc.projectInfo.mockResolvedValue(null);
  ipc.viewerBackend.mockResolvedValue("native");
  ipc.setViewerRect.mockResolvedValue(undefined);
});
afterEach(cleanup);

describe("App", () => {
  it("shows the app version and that no project is open", async () => {
    render(<App />);
    expect(await screen.findByText("Thira v1.2.3")).toBeDefined();
    expect(screen.getByText("No project open")).toBeDefined();
  });

  it("creates a new project", async () => {
    ipc.newProject.mockResolvedValue(info());
    render(<App />);
    await click("New");
    expect(ipc.newProject).toHaveBeenCalledWith(false);
    expect(screen.getByText("Untitled")).toBeDefined();
    expect(screen.getByText("Not saved")).toBeDefined();
  });

  it("asks for a file name the first time a project is saved", async () => {
    ipc.projectInfo.mockResolvedValue(info({ dirty: true }));
    dialog.save.mockResolvedValue("C:/films/Trip.thira");
    ipc.saveProjectAs.mockResolvedValue(info({ name: "Trip", path: "C:/films/Trip.thira" }));
    render(<App />);
    await screen.findByText("Untitled");
    await click("Save");
    expect(ipc.saveProject).not.toHaveBeenCalled();
    expect(ipc.saveProjectAs).toHaveBeenCalledWith("C:/films/Trip.thira");
    expect(screen.getByText("C:/films/Trip.thira")).toBeDefined();
  });

  it("asks before discarding unsaved changes, and retries with force", async () => {
    ipc.projectInfo.mockResolvedValue(info({ dirty: true }));
    ipc.closeProject
      .mockRejectedValueOnce({ code: "unsaved", message: "unsaved" })
      .mockResolvedValueOnce(undefined);
    dialog.ask.mockResolvedValue(true);
    render(<App />);
    await screen.findByText("Untitled");
    await click("Close");
    expect(ipc.closeProject.mock.calls).toEqual([[false], [true]]);
    expect(screen.getByText("No project open")).toBeDefined();
  });

  it("keeps the project when the user cancels the discard prompt", async () => {
    ipc.projectInfo.mockResolvedValue(info({ dirty: true }));
    ipc.closeProject.mockRejectedValue({ code: "unsaved", message: "unsaved" });
    dialog.ask.mockResolvedValue(false);
    render(<App />);
    await screen.findByText("Untitled");
    await click("Close");
    expect(ipc.closeProject).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Untitled")).toBeDefined();
  });

  it("shows engine errors", async () => {
    dialog.open.mockResolvedValue("C:/notes.thira");
    ipc.openProject.mockRejectedValue({
      code: "not_a_project",
      message: "not a Thira project file",
    });
    render(<App />);
    await click("Open…");
    expect(screen.getByRole("alert").textContent).toBe("not a Thira project file");
  });
});
