import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";

vi.mock("./ipc", () => ({ appVersion: () => Promise.resolve("1.2.3") }));

describe("App", () => {
  it("shows the app version from the shell", async () => {
    render(<App />);
    expect(await screen.findByText("v1.2.3")).toBeDefined();
  });
});
