import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

import { useToastStore } from "@/stores/toast-store";

import { useAppUpdateQuery } from "../data/useAppUpdate";

export function AppUpdateRuntime() {
  const { t } = useTranslation("settings");
  const pushToast = useToastStore((state) => state.push);
  const update = useAppUpdateQuery();
  const notified = useRef<string | null>(null);
  useEffect(() => {
    if (update.data && notified.current !== update.data.version) {
      notified.current = update.data.version;
      pushToast({
        level: "info",
        title: t("updateAvailable"),
        message: t("updateAvailableDescription", { version: update.data.version }),
      });
    }
  }, [pushToast, t, update.data]);
  return null;
}
