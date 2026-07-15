import {
  FolderCog,
  Image,
  KeyRound,
  MonitorCog,
  WandSparkles,
  type LucideIcon,
} from "lucide-react";
import { useCallback } from "react";

import { AppPanel } from "@/components/ui";

export type SettingsSection = "workspace" | "account" | "generation" | "images" | "frontend";

type SettingSection = {
  id: SettingsSection;
  label: string;
  description: string;
  icon: LucideIcon;
};

const applicationSections: ReadonlyArray<SettingSection> = [
  {
    id: "frontend",
    label: "Interface",
    description: "Preferences shared across workspaces",
    icon: MonitorCog,
  },
];

const workspaceSections: ReadonlyArray<SettingSection> = [
  {
    id: "workspace",
    label: "Workspace",
    description: "Current workspace and lifecycle",
    icon: FolderCog,
  },
  {
    id: "account",
    label: "Account",
    description: "NovelAI API keys and subscription probes",
    icon: KeyRound,
  },
  {
    id: "generation",
    label: "Generation",
    description: "Workspace defaults for image requests",
    icon: WandSparkles,
  },
  {
    id: "images",
    label: "Images",
    description: "Stored thumbnail and preview variants",
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
  return (
    <AppPanel
      as="nav"
      variant="section"
      aria-label="Settings sections"
      className="min-h-0 overflow-hidden"
    >
      <div className="grid gap-1 p-2">
        <SettingsSectionGroup
          label="Application"
          sections={applicationSections}
          activeSection={activeSection}
          onSelect={onSelect}
        />
        <SettingsSectionGroup
          label="Workspace"
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
  const Icon = section.icon;
  const handleClick = useCallback(() => {
    onSelect(section.id);
  }, [onSelect, section.id]);

  return (
    <button
      type="button"
      aria-label={section.label}
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
        <span className="block text-sm font-semibold">{section.label}</span>
        <span className="mt-1 block text-xs leading-4 text-app-muted">{section.description}</span>
      </span>
    </button>
  );
}
