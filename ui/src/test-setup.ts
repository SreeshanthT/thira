// Runs before every test file (vite.config.ts).
import "./i18n";

// jsdom has no layout, so it lacks ResizeObserver, which the panel layout
// needs. Nothing is ever resized in tests, so a no-op stub is enough.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};
