import js from "@eslint/js";
import i18next from "eslint-plugin-i18next";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "**/target", "app/gen", "spikes"] },
  {
    files: ["**/*.{ts,tsx}"],
    extends: [
      js.configs.recommended,
      ...tseslint.configs.strict,
      reactHooks.configs.flat.recommended,
    ],
    languageOptions: { globals: globals.browser },
  },
  {
    // No network access from the UI (PRD privacy); the CSP also blocks it.
    files: ["ui/src/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-globals": [
        "error",
        ...["fetch", "XMLHttpRequest", "WebSocket", "EventSource", "WebTransport"].map((name) => ({
          name,
          message: "Thira makes no network calls.",
        })),
      ],
      "no-restricted-properties": [
        "error",
        { object: "navigator", property: "sendBeacon", message: "Thira makes no network calls." },
        { object: "window", property: "fetch", message: "Thira makes no network calls." },
      ],
    },
  },
  {
    // UI text must come from ui/src/locales (PRD APP-3): every string literal
    // is an error except these technical ones.
    files: ["ui/src/**/*.{ts,tsx}"],
    ignores: ["**/*.test.{ts,tsx}", "ui/src/i18n.ts"],
    plugins: { i18next },
    rules: {
      "i18next/no-literal-string": [
        "error",
        {
          mode: "all",
          callees: {
            // Defaults, plus IPC command names, canvas context names and
            // developer-facing errors.
            exclude: [
              "i18n(ext)?",
              "t",
              "require",
              "addEventListener",
              "removeEventListener",
              "getElementById",
              "includes",
              "startsWith",
              "endsWith",
              "invoke",
              "getContext",
              "Error",
            ],
          },
          "object-properties": { exclude: ["[A-Z_-]+", "kind", "id"] },
          "jsx-attributes": {
            // Defaults, plus layout orientation.
            exclude: [
              "className",
              "styleName",
              "style",
              "type",
              "key",
              "id",
              "width",
              "height",
              "orientation",
            ],
          },
        },
      ],
    },
  },
);
