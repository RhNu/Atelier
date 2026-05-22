import { AppPanel } from "../../../components/ui";
import { SectionHeader, TextField } from "./SettingsControls";

function ignoreReadonlyPreference() {
  return undefined;
}

export function FrontendSettingsSection() {
  return (
    <AppPanel className="h-full min-h-0 overflow-hidden">
      <SectionHeader
        kicker="Frontend"
        title="Frontend Preferences"
        description="Reserved for future app-api backed UI preferences."
      />
      <div className="grid gap-3 p-3 md:grid-cols-3">
        <TextField label="Theme" value="System" onChange={ignoreReadonlyPreference} disabled />
        <TextField
          label="Density"
          value="Comfortable"
          onChange={ignoreReadonlyPreference}
          disabled
        />
        <TextField label="Language" value="System" onChange={ignoreReadonlyPreference} disabled />
      </div>
      <p className="border-t border-app-border px-3 py-3 text-sm text-app-muted">
        Frontend preferences are intentionally read-only until a workspace settings contract owns
        them.
      </p>
    </AppPanel>
  );
}
