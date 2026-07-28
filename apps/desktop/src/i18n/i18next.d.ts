import "i18next";
import type { en } from "./locales";

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "common";
    resources: typeof en;
    returnNull: false;
  }
}
