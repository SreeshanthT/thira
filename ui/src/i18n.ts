import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";

/**
 * UI text lives in `locales/<language>.json` (PRD APP-3); components get it
 * with `useTranslation()`, other modules with `i18n.t`. English is the only
 * language so far, and the source of the key types below.
 */
export const defaultNS = "translation";
export const resources = { en: { translation: en } } as const;

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: typeof defaultNS;
    resources: (typeof resources)["en"];
  }
}

// Resources are bundled, so initialization completes synchronously.
void i18n.use(initReactI18next).init({
  lng: "en",
  fallbackLng: "en",
  resources,
  initAsync: false,
  // React already escapes text it renders.
  interpolation: { escapeValue: false },
});

export default i18n;
