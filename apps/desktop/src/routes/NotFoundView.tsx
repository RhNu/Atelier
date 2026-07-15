import { useTranslation } from "react-i18next";

import { EmptyState } from "@/components/ui";

export function NotFoundView() {
  const { t } = useTranslation("shell");
  return <EmptyState title={t("viewNotFound")} />;
}
