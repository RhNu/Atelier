/* eslint-disable max-lines */
import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { AppToastHost } from "../components/ui";
import { SettingsPage } from "../features/settings";
import type {
  ApiKeyRecordDto,
  CreateApiKeyRequestDto,
  DeleteApiKeyRequestDto,
  DeleteApiKeyResponseDto,
  DanbooruAccountDto,
  SaveDanbooruAccountRequestDto,
  GlobalSettingsDto,
  DownloadableResourceGroupRequestDto,
  DownloadableResourceInstallProgressDto,
  DownloadableResourceRequestDto,
  DownloadableResourceStatusDto,
  DownloadableResourcesDto,
  ResetWorkspaceSettingsResponseDto,
  SetActiveApiKeyRequestDto,
  SubscriptionSummaryDto,
  UpdateApiKeyRequestDto,
  UpdateGlobalSettingsRequestDto,
  UpdateWorkspaceSettingsRequestDto,
  WorkspaceSettingsDto,
} from "../types";

const mocks = vi.hoisted(() => ({
  closeWorkspace: vi.fn<() => void>(),
  accountApi: {
    create: vi.fn<(request: CreateApiKeyRequestDto) => Promise<ApiKeyRecordDto>>(),
    update: vi.fn<(request: UpdateApiKeyRequestDto) => Promise<ApiKeyRecordDto>>(),
    delete: vi.fn<(request: DeleteApiKeyRequestDto) => Promise<DeleteApiKeyResponseDto>>(),
    list: vi.fn<() => Promise<ApiKeyRecordDto[]>>(),
    setActive: vi.fn<(request: SetActiveApiKeyRequestDto) => Promise<void>>(),
    probeActive: vi.fn<() => Promise<SubscriptionSummaryDto>>(),
  },
  danbooruApi: {
    account: vi.fn<() => Promise<DanbooruAccountDto>>(),
    saveAccount: vi.fn<(request: SaveDanbooruAccountRequestDto) => Promise<DanbooruAccountDto>>(),
    probeAccount: vi.fn<() => Promise<DanbooruAccountDto>>(),
    deleteAccount: vi.fn<() => Promise<DanbooruAccountDto>>(),
  },
  settingsApi: {
    get: vi.fn<() => Promise<WorkspaceSettingsDto>>(),
    update: vi.fn<(request: UpdateWorkspaceSettingsRequestDto) => Promise<WorkspaceSettingsDto>>(),
    reset: vi.fn<() => Promise<ResetWorkspaceSettingsResponseDto>>(),
  },
  globalSettingsApi: {
    get: vi.fn<() => Promise<GlobalSettingsDto>>(),
    update: vi.fn<(request: UpdateGlobalSettingsRequestDto) => Promise<GlobalSettingsDto>>(),
  },
  downloadableResourcesApi: {
    list: vi.fn<() => Promise<DownloadableResourcesDto>>(),
    refresh: vi.fn<() => Promise<DownloadableResourcesDto>>(),
    completeOnboarding: vi.fn<() => Promise<void>>(),
    install:
      vi.fn<
        (
          request: DownloadableResourceRequestDto,
          onProgress: (progress: DownloadableResourceInstallProgressDto) => void,
        ) => Promise<DownloadableResourceStatusDto>
      >(),
    installGroup:
      vi.fn<
        (
          request: DownloadableResourceGroupRequestDto,
          onProgress: (progress: DownloadableResourceInstallProgressDto) => void,
        ) => Promise<DownloadableResourceStatusDto[]>
      >(),
    cancelInstall: vi.fn<(request: DownloadableResourceRequestDto) => Promise<void>>(),
    delete: vi.fn<(request: DownloadableResourceRequestDto) => Promise<void>>(),
  },
}));

vi.mock("../features/workspace/useWorkspaceStatus", () => ({
  useWorkspaceStatus: () => ({
    workspaceStatus: { root: "D:/atelier", schema_version: 1, locked: true },
    workspacePending: false,
    workspaceErrorCode: undefined,
    workspaceErrorMessage: undefined,
    restoreFailure: null,
    openWorkspace: vi.fn<() => void>(),
    retryWorkspaceRestore: vi.fn<() => void>(),
    closeWorkspace: mocks.closeWorkspace,
    openingWorkspace: false,
    closingWorkspace: false,
  }),
}));

vi.mock("../features/generation/data/useImageModelCatalog", () => {
  const data = [
    { model: "nai-diffusion-5-full", capabilities: { default_scale: 7 } },
    { model: "nai-diffusion-4-5-full", capabilities: { default_scale: 5 } },
    { model: "nai-diffusion-3", capabilities: { default_scale: 5 } },
  ];
  return {
    useImageModelCatalog: () => ({ data }),
    findModelDescriptor: (catalog: typeof data, model: string) =>
      catalog.find((descriptor) => descriptor.model === model),
  };
});

vi.mock("../platform/atelier", () => ({
  accountApi: mocks.accountApi,
  danbooruApi: mocks.danbooruApi,
  settingsApi: mocks.settingsApi,
  globalSettingsApi: mocks.globalSettingsApi,
  downloadableResourcesApi: mocks.downloadableResourcesApi,
  appUpdateApi: {
    check: vi.fn<() => Promise<null>>(),
    install: vi.fn<() => Promise<void>>(),
  },
  queryKeys: {
    app: {
      bootstrap: () => ["app", "bootstrap"],
      globalSettings: () => ["app", "settings"],
      downloadableResources: () => ["app", "downloadable-resources"],
      appUpdate: () => ["app", "update"],
    },
    account: {
      root: () => ["account"],
      apiKeys: () => ["account", "api-keys"],
      activeSummary: () => ["account", "active-summary"],
    },
    settings: {
      root: () => ["settings"],
      workspace: () => ["settings", "workspace"],
    },
    danbooru: {
      root: () => ["app", "danbooru"],
      account: () => ["app", "danbooru", "account"],
    },
  },
}));

const defaultSettings: WorkspaceSettingsDto = {
  generation: {
    model: "nai-diffusion-4-5-full",
    size: { width: 832, height: 1216 },
    quality: "standard",
    transparent_background: false,
    uc_preset: "heavy",
    steps: 23,
    scale: 5,
    sampler: "k_euler",
    noise_schedule: "karras",
    seed: 0,
    n_samples: 1,
    cfg_rescale: 0,
    variety_boost: false,
    image_format: null,
    strict_mode: false,
  },
  image_variants: {
    thumbnail_long_edge: 320,
    preview_long_edge: 1024,
  },
};

const defaultGlobalSettings: GlobalSettingsDto = {
  last_workspace: "D:/atelier",
  frontend: {
    language: "system",
    developer_mode: false,
    gallery: {
      blur_sensitive_images: false,
    },
  },
  safety: { wd_auto_review_enabled: false },
};

const activeSubscription: SubscriptionSummaryDto = {
  anlas_balance: 1234,
  v5_usage: null,
  is_opus: true,
  subscription_active: true,
  tier: 3,
  tier_name: "Opus",
  expires_at_ms: 1_800_000_000_000,
};

const readyResources: DownloadableResourcesDto = {
  catalog_version: "1.0.0",
  onboarding_complete: true,
  groups: [],
  resources: [
    {
      id: "anime-dbrating",
      state: "ready",
      available_version: "1.0.0",
      installed_version: "1.0.0",
      size_bytes: 16_832_853,
      downloaded_bytes: 16_832_853,
      message: null,
    },
    {
      id: "wd-swinv2-tagger-v3",
      state: "ready",
      available_version: "1.0.0",
      installed_version: "1.0.0",
      size_bytes: 467_769_446,
      downloaded_bytes: 467_769_446,
      message: null,
    },
  ],
};

function cloneSettings(): WorkspaceSettingsDto {
  return structuredClone(defaultSettings) as WorkspaceSettingsDto;
}

function lastWorkspaceSettingsUpdate(): UpdateWorkspaceSettingsRequestDto {
  const request = mocks.settingsApi.update.mock.calls.at(-1)?.[0];
  if (!request) {
    throw new Error("Expected settingsApi.update to be called.");
  }

  return request;
}

function lastGlobalSettingsUpdate(): UpdateGlobalSettingsRequestDto {
  const request = mocks.globalSettingsApi.update.mock.calls.at(-1)?.[0];
  if (!request) {
    throw new Error("Expected globalSettingsApi.update to be called.");
  }
  return request;
}

function setup(
  keys: ApiKeyRecordDto[] = [],
  danbooruConfigured = false,
  subscription = activeSubscription,
) {
  mocks.settingsApi.get.mockResolvedValue(cloneSettings());
  mocks.settingsApi.update.mockImplementation(async ({ settings }) => settings);
  mocks.settingsApi.reset.mockResolvedValue({ settings: cloneSettings() });
  mocks.globalSettingsApi.get.mockResolvedValue(structuredClone(defaultGlobalSettings));
  mocks.globalSettingsApi.update.mockImplementation(async ({ frontend, safety }) => ({
    last_workspace: "D:/atelier",
    frontend,
    safety,
  }));
  mocks.downloadableResourcesApi.list.mockResolvedValue(readyResources);
  mocks.downloadableResourcesApi.refresh.mockResolvedValue(readyResources);
  mocks.downloadableResourcesApi.completeOnboarding.mockResolvedValue(undefined);
  mocks.downloadableResourcesApi.cancelInstall.mockResolvedValue(undefined);
  mocks.downloadableResourcesApi.delete.mockResolvedValue(undefined);
  mocks.accountApi.list.mockResolvedValue(keys);
  mocks.accountApi.create.mockImplementation(async (request) => ({
    id: request.id,
    display_name: request.display_name,
    is_active: false,
  }));
  mocks.accountApi.update.mockImplementation(async (request) => ({
    id: request.id,
    display_name: request.display_name ?? "Updated key",
    is_active: false,
  }));
  mocks.accountApi.delete.mockResolvedValue({ deleted: true });
  mocks.accountApi.setActive.mockResolvedValue(undefined);
  mocks.accountApi.probeActive.mockResolvedValue(subscription);
  mocks.danbooruApi.account.mockResolvedValue({
    configured: danbooruConfigured,
    state: danbooruConfigured ? "configured" : "anonymous",
    username: danbooruConfigured ? "atelier-user" : null,
    level: null,
  });
  mocks.danbooruApi.saveAccount.mockImplementation(async (request) => ({
    configured: true,
    state: "configured",
    username: request.username,
    level: null,
  }));
  mocks.danbooruApi.probeAccount.mockResolvedValue({
    configured: true,
    state: "verified",
    username: "atelier-user",
    level: "2",
  });
  mocks.danbooruApi.deleteAccount.mockResolvedValue({
    configured: false,
    state: "anonymous",
    username: null,
    level: null,
  });

  return {
    user: userEvent.setup(),
    ...render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <SettingsPage />
        <AppToastHost />
      </QueryClientProvider>,
    ),
  };
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.clearAllMocks();
  vi.spyOn(crypto, "randomUUID").mockReturnValue("00000000-0000-4000-8000-000000000001");
});

describe("SettingsPage", () => {
  it("uses a settings section navigator instead of one mixed settings page", async () => {
    const { user } = setup();

    const sectionNav = await screen.findByRole("navigation", { name: "Settings sections" });
    expect(within(sectionNav).getByRole("button", { name: "Connections" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(within(sectionNav).queryByRole("button", { name: "Account" })).not.toBeInTheDocument();
    expect(within(sectionNav).getByRole("button", { name: "Workspace" })).toBeInTheDocument();
    expect(within(sectionNav).getByRole("button", { name: "Generation" })).toBeInTheDocument();
    expect(within(sectionNav).getByRole("button", { name: "Images" })).toBeInTheDocument();
    expect(within(sectionNav).getByRole("button", { name: "Interface" })).toBeInTheDocument();
    expect(within(sectionNav).getByRole("button", { name: "Safety" })).toBeInTheDocument();

    await user.click(within(sectionNav).getByRole("button", { name: "Interface" }));

    expect(screen.getByRole("heading", { name: "Interface" })).toBeInTheDocument();
    expect(screen.getAllByText("Language")).toHaveLength(1);

    expect(screen.getByLabelText("Developer mode")).toBeInTheDocument();
    expect(screen.getByLabelText("Blur NSFW images")).toBeInTheDocument();
  });
});

describe("Safety settings", () => {
  it("enables WD automatic review only with a ready optional model", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Safety" }));
    expect(screen.getByText("WD SwinV2 Tagger v3")).toBeInTheDocument();
    const toggle = screen.getByLabelText("WD automatic review");
    expect(toggle).toBeEnabled();
    await user.click(toggle);

    await waitFor(() =>
      expect(lastGlobalSettingsUpdate().safety.wd_auto_review_enabled).toBe(true),
    );
  });

  it("keeps automatic review disabled while the WD resource is unavailable", async () => {
    const { user } = setup();
    mocks.downloadableResourcesApi.list.mockResolvedValue({
      ...readyResources,
      resources: readyResources.resources.map((resource) =>
        resource.id === "wd-swinv2-tagger-v3"
          ? { ...resource, state: "missing", installed_version: null }
          : resource,
      ),
    });

    await user.click(await screen.findByRole("button", { name: "Safety" }));
    expect(await screen.findByLabelText("WD automatic review")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Download" })).toBeInTheDocument();
  });
});

describe("Opus allowance", () => {
  it("shows the generation allowance and time to refill when usage is available", async () => {
    setup([{ id: "main", display_name: "Main", is_active: true }], false, {
      ...activeSubscription,
      v5_usage: {
        is_negative: false,
        percent: 62,
        seconds_until_next_percent: 7888,
      },
    });

    expect(await screen.findByText("Opus generations")).toBeInTheDocument();
    expect(screen.getByText("62% · ~83h")).toBeInTheDocument();
  });
});

describe("SettingsPage commands", () => {
  it("keeps Danbooru credentials inside a configuration modal", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Connections" }));
    const novelAi = await screen.findByRole("heading", { name: "NovelAI" });
    const danbooru = screen.getByRole("heading", { name: "Danbooru" });
    expect(
      novelAi.compareDocumentPosition(danbooru) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(screen.getByText("Optional")).toBeInTheDocument();
    expect(screen.queryByLabelText("Danbooru username")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Configure connection" }));
    expect(screen.getByRole("dialog", { name: "Configure connection" })).toBeInTheDocument();
    await user.type(screen.getByLabelText("Danbooru username"), "atelier-user");
    await user.type(screen.getByLabelText("Danbooru API key"), "danbooru-secret");
    await user.click(screen.getByRole("button", { name: "Save connection" }));

    expect(mocks.danbooruApi.saveAccount).toHaveBeenCalledWith({
      username: "atelier-user",
      api_key: "danbooru-secret",
    });
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: "Configure connection" }),
      ).not.toBeInTheDocument(),
    );
    expect(screen.queryByText("danbooru-secret")).not.toBeInTheDocument();
  });

  it("reports Danbooru test results through a toast without rendering verification state", async () => {
    const { user } = setup([], true);

    await user.click(await screen.findByRole("button", { name: "Connections" }));
    await user.click(screen.getByRole("button", { name: "Test connection" }));

    expect(await screen.findByText("Danbooru account verified (2).")).toBeInTheDocument();
    expect(screen.queryByText("Verified")).not.toBeInTheDocument();
    expect(screen.queryByText("Configured · unverified")).not.toBeInTheDocument();
  });

  it("creates API keys with generated ids and never displays the secret", async () => {
    const { user } = setup();

    await screen.findByText("No API keys");
    expect(screen.queryByLabelText("API key display name")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Add API key" }));
    expect(screen.getByRole("dialog", { name: "Add API key" })).toBeInTheDocument();
    await user.type(screen.getByLabelText("API key display name"), "Main key");
    await user.type(screen.getByLabelText("NovelAI API key secret"), "nai-secret-value");
    await user.click(screen.getByRole("button", { name: "Add" }));

    expect(mocks.accountApi.create).toHaveBeenCalledWith({
      id: "00000000-0000-4000-8000-000000000001",
      display_name: "Main key",
      secret: "nai-secret-value",
    });
    expect(screen.queryByDisplayValue("nai-secret-value")).not.toBeInTheDocument();
    expect(screen.queryByText("nai-secret-value")).not.toBeInTheDocument();
  });

  it("discards an unfinished API key when the creation dialog is cancelled", async () => {
    const { user } = setup();

    await screen.findByText("No API keys");
    await user.click(screen.getByRole("button", { name: "Add API key" }));
    await user.type(screen.getByLabelText("API key display name"), "Discard me");
    await user.type(screen.getByLabelText("NovelAI API key secret"), "temporary-secret");
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await user.click(screen.getByRole("button", { name: "Add API key" }));

    expect(screen.getByLabelText("API key display name")).toHaveValue("");
    expect(screen.getByLabelText("NovelAI API key secret")).toHaveValue("");
  });

  it("lists API keys and supports activate, edit, and delete actions", async () => {
    const { user } = setup([
      { id: "main", display_name: "Main", is_active: true },
      { id: "backup", display_name: "Backup", is_active: false },
    ]);

    expect(await screen.findByText("Main")).toBeInTheDocument();
    expect(screen.getByText("Active")).toBeInTheDocument();
    expect(screen.queryByText("ID main")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Set Backup active" }));
    expect(mocks.accountApi.setActive).toHaveBeenCalledWith({ id: "backup" });

    await user.click(screen.getByRole("button", { name: "Edit Backup" }));
    expect(screen.getByRole("dialog", { name: "Edit Backup" })).toBeInTheDocument();
    expect(screen.getByLabelText("Replace API key secret")).toHaveValue("");
    await user.clear(screen.getByLabelText("Edit API key display name"));
    await user.type(screen.getByLabelText("Edit API key display name"), "Backup edited");
    await user.type(screen.getByLabelText("Replace API key secret"), "replacement-secret");
    await user.click(screen.getByRole("button", { name: "Save API key changes" }));
    expect(mocks.accountApi.update).toHaveBeenCalledWith({
      id: "backup",
      display_name: "Backup edited",
      secret: "replacement-secret",
    });
    expect(screen.queryByDisplayValue("replacement-secret")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Delete Backup" }));
    expect(mocks.accountApi.delete).toHaveBeenCalledWith({ id: "backup" });
  });

  it("loads active subscription status automatically without a refresh control", async () => {
    setup([{ id: "main", display_name: "Main", is_active: true }]);
    expect(mocks.accountApi.probeActive).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("Opus")).toBeInTheDocument();
    expect(screen.getByText("1234 Anlas")).toBeInTheDocument();
    expect(screen.queryByText("Opus access")).not.toBeInTheDocument();
    expect(screen.queryByText("Subscription information is up to date.")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /subscription/i })).not.toBeInTheDocument();
  });

  it("reports API key mutation failures without echoing the secret", async () => {
    mocks.accountApi.create.mockRejectedValueOnce(new Error("Keyring unavailable"));
    const { user } = setup();

    await screen.findByText("No API keys");
    await user.click(screen.getByRole("button", { name: "Add API key" }));
    await user.type(screen.getByLabelText("API key display name"), "Main key");
    await user.type(screen.getByLabelText("NovelAI API key secret"), "nai-secret-value");
    await user.click(screen.getByRole("button", { name: "Add" }));

    expect(await screen.findByText("Keyring unavailable")).toBeInTheDocument();
    expect(screen.queryByText("nai-secret-value")).not.toBeInTheDocument();
  });

  it("invalidates the active summary after replacing a key", async () => {
    const { user } = setup([{ id: "main", display_name: "Main", is_active: true }]);

    expect(await screen.findByText("Main")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Edit Main" }));
    await user.type(screen.getByLabelText("Replace API key secret"), "replacement-secret");
    await user.click(screen.getByRole("button", { name: "Save API key changes" }));

    expect(mocks.accountApi.update).toHaveBeenCalledTimes(1);
  });

  it("saves generation defaults through the workspace settings command", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Generation" }));
    await user.click(screen.getByLabelText("Model"));
    await user.click(screen.getByRole("option", { name: "NAI Diffusion 3" }));
    await user.clear(screen.getByLabelText("Width"));
    await user.type(screen.getByLabelText("Width"), "1024");
    await user.clear(screen.getByLabelText("Steps"));
    await user.type(screen.getByLabelText("Steps"), "28");
    await user.click(screen.getByLabelText("Variety boost"));
    await waitFor(() => expect(mocks.settingsApi.update).toHaveBeenCalled());

    const request = lastWorkspaceSettingsUpdate();
    expect(request.settings.generation.model).toBe("nai-diffusion-3");
    expect(request.settings.generation.size).toEqual({ width: 1024, height: 1216 });
    expect(request.settings.generation.steps).toBe(28);
    expect(request.settings.generation.variety_boost).toBe(true);
  });

  it("saves image variant sizes only when values are positive integers", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Images" }));
    await user.clear(screen.getByLabelText("Thumbnail long edge"));
    await user.type(screen.getByLabelText("Thumbnail long edge"), "0");

    expect(screen.queryByRole("button", { name: "Save image variants" })).not.toBeInTheDocument();
    expect(screen.getByText("Image variant sizes must be positive integers.")).toBeInTheDocument();

    await user.clear(screen.getByLabelText("Thumbnail long edge"));
    await user.type(screen.getByLabelText("Thumbnail long edge"), "640");
    await user.clear(screen.getByLabelText("Preview long edge"));
    await user.type(screen.getByLabelText("Preview long edge"), "1600");
    await waitFor(() =>
      expect(lastWorkspaceSettingsUpdate().settings.image_variants).toEqual({
        thumbnail_long_edge: 640,
        preview_long_edge: 1600,
      }),
    );
  });

  it("does not persist invalid image draft values when saving generation", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Images" }));
    await user.clear(screen.getByLabelText("Thumbnail long edge"));
    await user.type(screen.getByLabelText("Thumbnail long edge"), "0");

    await user.click(screen.getByRole("button", { name: "Generation" }));
    await user.click(screen.getByLabelText("Model"));
    await user.click(screen.getByRole("option", { name: "NAI Diffusion 3" }));
    await waitFor(() => expect(mocks.settingsApi.update).toHaveBeenCalled());

    const request = lastWorkspaceSettingsUpdate();
    expect(request.settings.generation.model).toBe("nai-diffusion-3");
    expect(request.settings.image_variants.thumbnail_long_edge).toBe(320);
  });

  it("reports workspace settings save failures", async () => {
    mocks.settingsApi.update.mockRejectedValueOnce(new Error("Settings backend unavailable"));
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Generation" }));
    await user.click(screen.getByLabelText("Model"));
    await user.click(screen.getByRole("option", { name: "NAI Diffusion 3" }));
    expect(await screen.findByText("Settings backend unavailable")).toBeInTheDocument();
  });

  it("resets workspace settings to backend defaults", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Generation" }));
    await user.click(screen.getByRole("button", { name: "Reset workspace settings" }));

    expect(mocks.settingsApi.reset).toHaveBeenCalledTimes(1);
  });

  it("saves frontend preferences through global settings", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Interface" }));
    await user.click(screen.getByLabelText("Developer mode"));
    await user.click(screen.getByLabelText("Blur NSFW images"));
    await waitFor(() => expect(mocks.globalSettingsApi.update).toHaveBeenCalled());

    const request = lastGlobalSettingsUpdate();
    expect(request.frontend.developer_mode).toBe(true);
    expect(request.frontend.gallery.blur_sensitive_images).toBe(true);
    expect(mocks.settingsApi.update).not.toHaveBeenCalled();
  });

  it("closes the current workspace from the workspace settings section", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Workspace" }));
    expect(screen.getByText("D:/atelier")).toBeInTheDocument();
    expect(screen.queryByText("Schema version")).not.toBeInTheDocument();
    expect(screen.queryByText("Workspace lock")).not.toBeInTheDocument();
    await user.click(await screen.findByRole("button", { name: "Close workspace" }));

    expect(mocks.closeWorkspace).toHaveBeenCalledTimes(1);
  });
});
