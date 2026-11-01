import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Viewer } from "./Viewer";

const ipc = vi.hoisted(() => ({
  viewerBackend: vi.fn(),
  setViewerRect: vi.fn(),
  viewerFrame: vi.fn(),
}));
vi.mock("./ipc", () => ipc);

beforeEach(() => {
  vi.resetAllMocks();
  ipc.setViewerRect.mockResolvedValue(undefined);
  delete document.documentElement.dataset.viewer;
});
afterEach(cleanup);

async function mount() {
  const result = render(<Viewer />);
  // Let the backend query resolve and effects run.
  await act(async () => undefined);
  return result;
}

describe("Viewer", () => {
  it("reports its rectangle in physical pixels for the native backend", async () => {
    ipc.viewerBackend.mockResolvedValue("native");
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      left: 10,
      top: 40.4,
      width: 640,
      height: 360,
    } as DOMRect);
    vi.stubGlobal("devicePixelRatio", 1.5);

    const { unmount } = await mount();
    expect(document.documentElement.dataset.viewer).toBe("native");
    expect(ipc.setViewerRect).toHaveBeenCalledWith({ x: 15, y: 61, width: 960, height: 540 });

    unmount();
    expect(ipc.setViewerRect).toHaveBeenLastCalledWith(null);
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows a canvas for the transfer backend and never reports a rectangle", async () => {
    ipc.viewerBackend.mockResolvedValue("transfer");
    const { container } = await mount();
    expect(document.documentElement.dataset.viewer).toBe("transfer");
    expect(container.querySelector("canvas")).not.toBeNull();
    expect(ipc.setViewerRect).not.toHaveBeenCalled();
  });

  it("renders an empty viewer outside the app shell", async () => {
    ipc.viewerBackend.mockRejectedValue(new Error("not in Tauri"));
    const { container } = await mount();
    expect(container.querySelector(".viewer")).not.toBeNull();
    expect(document.documentElement.dataset.viewer).toBeUndefined();
  });
});
