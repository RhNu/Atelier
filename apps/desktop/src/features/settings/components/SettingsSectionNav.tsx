import { Image, KeyRound, MonitorCog, WandSparkles, type LucideIcon } from "lucide-react";
import { useCallback } from "react";

import { AppPanel } from "../../../components/ui";

export type SettingsSection = "account" | "generation" | "images" | "frontend";

const settingSections: ReadonlyArray<{
  id: SettingsSection;
  label: string;
  description: string;
  icon: LucideIcon;
}> = [
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
  {
    id: "frontend",
    label: "Frontend",
    description: "Future app-facing preferences",
    icon: MonitorCog,
  },
] as const;

export function SettingsSectionNav({
  activeSection,
  onSelect,
}: {
  activeSection: SettingsSection;
  onSelect: (section: SettingsSection) => void;
}) {
  return (
    <AppPanel as="nav" aria-label="Settings sections" className="min-h-0 overflow-hidden">
      <div className="grid gap-1 p-2">
        {settingSections.map((section) => (
          <SettingsSectionButton
            key={section.id}
            section={section}
            active={section.id === activeSection}
            onSelect={onSelect}
          />
        ))}
      </div>
    </AppPanel>
  );
}

function SettingsSectionButton({
  section,
  active,
  onSelect,
}: {
  section: (typeof settingSections)[number];
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
