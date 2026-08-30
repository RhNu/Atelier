import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { ResourceOnboarding } from "../features/settings/components/ResourceOnboarding";
import type {
  DownloadableResourceGroupRequestDto,
  DownloadableResourceInstallProgressDto,
  DownloadableResourceStatusDto,
  DownloadableResourcesDto,
} from "../types";

const mocks = vi.hoisted(() => ({
  list: vi.fn<() => Promise<DownloadableResourcesDto>>(),
  completeOnboarding: vi.fn<() => Promise<void>>(),
  installGroup:
    vi.fn<
      (
        request: DownloadableResourceGroupRequestDto,
        onProgress: (progress: DownloadableResourceInstallProgressDto) => void,
      ) => Promise<DownloadableResourceStatusDto[]>
    >(),
}));

vi.mock("../platform/atelier", () => ({
  downloadableResourcesApi: {
    list: mocks.list,
    completeOnboarding: mocks.completeOnboarding,
    installGroup: mocks.installGroup,
  },
  queryKeys: {
    app: { downloadableResources: () => ["app", "downloadable-resources"] },
  },
}));

const catalog: DownloadableResourcesDto = {
  catalog_version: "1.0.0",
  onboarding_complete: false,
  resources: [],
  groups: [
    { id: "starter", resources: ["lexicon-core", "anime-dbrating"], size_bytes: 102_000_000 },
    {
      id: "semantic-search",
      resources: ["lexicon-core", "lexicon-semantic"],
      size_bytes: 284_000_000,
    },
    {
      id: "enhanced-safety",
      resources: ["anime-dbrating", "wd-swinv2-tagger-v3"],
      size_bytes: 484_000_000,
    },
  ],
};

function renderOnboarding() {
  return {
    user: userEvent.setup(),
    ...render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <ResourceOnboarding />
      </QueryClientProvider>,
    ),
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.list.mockResolvedValue(catalog);
  mocks.completeOnboarding.mockResolvedValue(undefined);
  mocks.installGroup.mockResolvedValue([]);
});

describe("ResourceOnboarding", () => {
  it("selects only the starter group by default and installs it explicitly", async () => {
    const { user } = renderOnboarding();
    expect(await screen.findByRole("dialog", { name: "Install Atelier resources" })).toBeVisible();
    expect(screen.getByRole("checkbox", { name: "Starter resources" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Semantic search" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Enhanced safety" })).not.toBeChecked();

    await user.click(screen.getByRole("button", { name: "Install selected" }));

    await waitFor(() =>
      expect(mocks.installGroup).toHaveBeenCalledWith(
        { group_id: "starter" },
        expect.any(Function),
      ),
    );
    expect(mocks.completeOnboarding).toHaveBeenCalledOnce();
  });

  it("can skip without starting a download", async () => {
    const { user } = renderOnboarding();
    await user.click(await screen.findByRole("button", { name: "Skip for now" }));
    await waitFor(() => expect(mocks.completeOnboarding).toHaveBeenCalledOnce());
    expect(mocks.installGroup).not.toHaveBeenCalled();
  });
});
