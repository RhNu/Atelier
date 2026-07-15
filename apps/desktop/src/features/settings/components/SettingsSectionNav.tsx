import {
  FolderCog,
  Image,
  KeyRound,
  MonitorCog,
  WandSparkles,
  type LucideIcon,
} from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { AppPanel } from "@/components/ui";

export type SettingsSection = "workspace" | "account" | "generation" | "images" | "frontend";

type SettingSection = {
  id: SettingsSection;
  labelKey: "interface" | "workspace" | "account" | "generation" | "images";
  descriptionKey:
    | "interfaceDescription"
    | "workspaceDescription"
    | "accountDescription"
    | "generationDescription"
    | "imagesDescription";
  icon: LucideIcon;
};

const applicationSections: ReadonlyArray<SettingSection> = [
  {
    id: "frontend",
    labelKey: "interface",
    descriptionKey: "interfaceDescription",
    icon: MonitorCog,
  },
];

const workspaceSections: ReadonlyArray<SettingSection> = [
  {
    id: "workspace",
    labelKey: "workspace",
    descriptionKey: "workspaceDescription",
    icon: FolderCog,
  },
  {
    id: "account",
    labelKey: "account",
    descriptionKey: "accountDescription",
    icon: KeyRound,
  },
  {
    id: "generation",
    labelKey: "generation",
    descriptionKey: "generationDescription",
    icon: WandSparkles,
  },
  {
    id: "images",
    labelKey: "images",
    descriptionKey: "imagesDescription",
    icon: Image,
  },
];

export function SettingsSectionNav({
  activeSection,
  onSelect,
}: {
  activeSection: SettingsSection;
  onSelect: (section: SettingsSection) => void;
}) {
  const { t } = useTranslation("settings");
  return (
    <AppPanel
      as="nav"
      variant="section"
      aria-label={t("sections")}
      className="min-h-0 overflow-hidden"
    >
      <div className="grid gap-1 p-2">
        <SettingsSectionGroup
          label={t("application")}
          sections={applicationSections}
          activeSection={activeSection}
          onSelect={onSelect}
        />
        <SettingsSectionGroup
          label={t("workspace")}
          sections={workspaceSections}
          activeSection={activeSection}
          onSelect={onSelect}
        />
      </div>
    </AppPanel>
  );
}

function SettingsSectionGroup({
  label,
  sections,
  activeSection,
  onSelect,
}: {
  label: string;
  sections: ReadonlyArray<SettingSection>;
  activeSection: SettingsSection;
  onSelect: (section: SettingsSection) => void;
}) {
  return (
    <div className="grid gap-1">
      <p className="px-3 pt-2 text-[10px] font-semibold tracking-wider text-app-muted uppercase">
        {label}
      </p>
      {sections.map((section) => (
        <SettingsSectionButton
          key={section.id}
          section={section}
          active={section.id === activeSection}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}

function SettingsSectionButton({
  section,
  active,
  onSelect,
}: {
  section: SettingSection;
  active: boolean;
  onSelect: (section: SettingsSection) => void;
}) {
  const { t } = useTranslation("settings");
  const Icon = section.icon;
  const label = t(section.labelKey);
  const handleClick = useCallback(() => {
    onSelect(section.id);
  }, [onSelect, section.id]);

  return (
    <button
      type="button"
      aria-label={label}
      aria-current={active ? "page" : undefined}
      className={[
        "flex min-h-14 items-start gap-3 border px-3 py-2 text-left transition-colors",
        active
          ? "border-brand-400/70 bg-brand-500/20 text-brand-100"
          : "border-transparent text-app-muted hover:bg-app-surface hover:text-app-text",
      ].join(" ")}
      onClick={handleClick}
    >
      <Icon aria-hidden="true" className="mt-0.5 size-4 shrink-0" />
      <span className="min-w-0">
        <span className="block text-sm font-semibold">{label}</span>
        <span className="mt-1 block text-xs leading-4 text-app-muted">
          {t(section.descriptionKey)}
        </span>
      </span>
    </button>
  );
}
