import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { layoutStorage, Workspace } from "./Workspace";

vi.mock("./ipc", () => ({
  viewerBackend: vi.fn(() => Promise.resolve("native")),
  setViewerRect: vi.fn(() => Promise.resolve()),
  viewerFrame: vi.fn(),
}));

const ROWS_KEY = "react-resizable-panels:thira.layout.v1.rows";

function grow(id: string) {
  const panel = document.getElementById(id);
  if (!panel) throw new Error(`no panel ${id}`);
  return Number(panel.style.flexGrow);
}

describe("Workspace", () => {
  beforeEach(() => {
    localStorage.clear();
    // jsdom does no layout; give every element a size so percentages resolve.
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue(
      DOMRect.fromRect({ width: 1000, height: 800 }),
    );
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(1000);
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockReturnValue(800);
  });
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("shows the four panels with resize handles between them", () => {
    render(<Workspace />);
    for (const name of ["Media Pool", "Viewer", "Inspector", "Timeline"]) {
      expect(screen.getByRole("region", { name })).toBeTruthy();
    }
    expect(screen.getAllByRole("separator", { name: "Resize panels" })).toHaveLength(3);
  });

  it("starts from the default layout", () => {
    render(<Workspace />);
    expect(grow("top")).toBe(60);
    expect(grow("timeline")).toBe(40);
    expect(grow("media")).toBe(20);
    expect(grow("viewer")).toBe(60);
    expect(grow("inspector")).toBe(20);
  });

  it("restores a saved layout", () => {
    localStorage.setItem(ROWS_KEY, JSON.stringify({ top: 70, timeline: 30 }));
    render(<Workspace />);
    expect(grow("top")).toBe(70);
    expect(grow("timeline")).toBe(30);
  });

  it("saves the layout when the user resizes with the keyboard", () => {
    render(<Workspace />);
    const [, , rowSeparator] = screen.getAllByRole("separator");
    if (!rowSeparator) throw new Error("missing separator");
    fireEvent.keyDown(rowSeparator, { key: "Home" });
    const saved: unknown = JSON.parse(localStorage.getItem(ROWS_KEY) ?? "null");
    expect(saved).toEqual({ top: 25, timeline: 75 });
  });

  it("does not save layouts it did not get from the user", () => {
    render(<Workspace />);
    expect(localStorage.length).toBe(0);
  });

  it("falls back to the default layout when storage fails", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("denied");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("denied");
    });
    expect(layoutStorage.getItem(ROWS_KEY)).toBeNull();
    expect(() => layoutStorage.setItem(ROWS_KEY, "{}")).not.toThrow();
    render(<Workspace />);
    expect(grow("top")).toBe(60);
  });
});
