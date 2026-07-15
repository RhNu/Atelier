import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { GalleryPage } from "../features/gallery";
import type {
  GalleryItemDto,
  GalleryPageDto,
  GalleryQueryDto,
  DeleteGalleryItemsRequestDto,
  DeleteGalleryItemsResponseDto,
  GetResourceImageRequestDto,
  GlobalSettingsDto,
  ResourceImageDto,
  SaveResourceImageRequestDto,
  SetGallerySafetyOverrideRequestDto,
} from "../types";

const mocks = vi.hoisted(() => ({
  galleryApi: {
    list: vi.fn<(request: GalleryQueryDto) => Promise<GalleryPageDto>>(),
    setSafetyOverride:
      vi.fn<(request: SetGallerySafetyOverrideRequestDto) => Promise<GalleryItemDto>>(),
    deleteItems:
      vi.fn<(request: DeleteGalleryItemsRequestDto) => Promise<DeleteGalleryItemsResponseDto>>(),
  },
  resourceApi: {
    image: vi.fn<(request: GetResourceImageRequestDto) => Promise<ResourceImageDto>>(),
  },
  desktopApi: {
    saveResourceImage:
      vi.fn<(request: SaveResourceImageRequestDto) => Promise<{ path: string } | null>>(),
  },
  globalSettingsApi: {
    get: vi.fn<() => Promise<GlobalSettingsDto>>(),
  },
}));

vi.mock("../platform/atelier", () => ({
  galleryApi: mocks.galleryApi,
  resourceApi: mocks.resourceApi,
  desktopApi: mocks.desktopApi,
  globalSettingsApi: mocks.globalSettingsApi,
  resourceImageToDataUrl: (image: ResourceImageDto) =>
    `data:${image.mime_type ?? "image/png"};base64,${image.image_base64}`,
  queryKeys: {
    gallery: {
      root: () => ["gallery"],
      list: (query: GalleryQueryDto) => ["gallery", "list", query],
    },
    resource: {
      root: () => ["resource"],
      image: (resource: { id: string; variant_id: string | null }) => [
        "resource",
        "image",
        resource,
      ],
    },
    history: {
      root: () => ["history"],
    },
    app: {
      globalSettings: () => ["app", "settings"],
    },
  },
}));

const defaultSettings: GlobalSettingsDto = {
  last_workspace: "D:/atelier",
  frontend: {
    language: "system",
    developer_mode: false,
    gallery: {
      blur_sensitive_images: false,
    },
  },
};

function galleryItem(
  itemId: string,
  options: {
    source: GalleryItemDto["source_kind"];
    artifactKind?: string;
    label?: "safe" | "sensitive" | "hidden";
    manualOverride?: "safe" | "sensitive" | "hidden" | null;
    resourceId?: string;
  },
): GalleryItemDto {
  const resourceId = options.resourceId ?? `resource:${itemId}:original`;
  const label = options.label ?? "safe";
  return {
    item_id: itemId,
    artifact_id: `artifact:${itemId}`,
    artifact_kind: options.artifactKind ?? "generated_image",
    source_kind: options.source,
    primary_resource: { id: resourceId, variant_id: null },
    assets: [
      {
        role: "original",
        resource: { id: resourceId, variant_id: null },
        variant_kind: "original",
      },
      {
        role: "thumbnail",
        resource: { id: resourceId, variant_id: "thumbnail" },
        variant_kind: "thumbnail",
      },
      {
        role: "preview",
        resource: { id: resourceId, variant_id: "preview" },
        variant_kind: "preview",
      },
    ],
    indexed_at_ms: 1_800_000_000_000,
    seed: 1234,
    sample_index: 0,
    model_name: "nai-diffusion-4-5-full",
    safety: {
      scan_state: "scanned",
      risk_band: label === "sensitive" ? "high" : "low",
      auto_label: label === "hidden" ? "sensitive" : label,
      effective_label: label,
      nsfw_score: label === "sensitive" || label === "hidden" ? 0.91 : 0.04,
      safe_score: label === "sensitive" || label === "hidden" ? 0.09 : 0.96,
      raw_scores: [
        { label: "safe", score: label === "safe" ? 0.96 : 0.09 },
        { label: "nsfw", score: label === "safe" ? 0.04 : 0.91 },
      ],
      model_id: "open_nsfw@onnx",
      scorer_version: "1",
      assessed_at_ms: 1_800_000_000_001,
    },
    manual_safety_override: options.manualOverride ?? null,
  };
}

function setup(options?: { blurSensitive?: boolean; items?: GalleryItemDto[] }) {
  const settings = structuredClone(defaultSettings) as GlobalSettingsDto;
  settings.frontend.gallery.blur_sensitive_images = options?.blurSensitive ?? false;
  mocks.globalSettingsApi.get.mockResolvedValue(settings);
  mocks.galleryApi.list.mockResolvedValue({
    items: options?.items ?? [
      galleryItem("safe-item", { source: "generation", label: "safe" }),
      galleryItem("sensitive-item", { source: "generation", label: "sensitive" }),
      galleryItem("hidden-item", {
        source: "director",
        artifactKind: "director_result",
        label: "hidden",
        manualOverride: "hidden",
      }),
    ],
    total: options?.items?.length ?? 3,
    offset: 0,
    limit: 24,
  });
  mocks.galleryApi.setSafetyOverride.mockImplementation(async (request) =>
    galleryItem(request.item_id, {
      source: "generation",
      label: request.manual_safety_override ?? "safe",
      manualOverride: request.manual_safety_override ?? null,
    }),
  );
  mocks.galleryApi.deleteItems.mockResolvedValue({
    deleted: 1,
    resources_released: 1,
    blobs_deleted: 3,
  });
  mocks.resourceApi.image.mockImplementation(async ({ resource }) => ({
    image_base64: `image-${resource.id}:${resource.variant_id ?? "base"}`,
    mime_type: "image/png",
  }));
  mocks.desktopApi.saveResourceImage.mockResolvedValue({ path: "C:\\exports\\gallery.png" });

  return {
    user: userEvent.setup(),
    ...render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <GalleryPage />
      </QueryClientProvider>,
    ),
  };
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.clearAllMocks();
});

describe("GalleryPage", () => {
  it("renders gallery images, hides manual hidden items by default, and shows item details", async () => {
    const { user } = setup();

    expect(await screen.findByText("safe-item")).toBeInTheDocument();
    expect(screen.getByText("sensitive-item")).toBeInTheDocument();
    expect(screen.queryByText("hidden-item")).not.toBeInTheDocument();
    expect(await screen.findByAltText("safe-item preview")).toHaveAttribute(
      "src",
      "data:image/png;base64,image-resource:safe-item:original:thumbnail",
    );

    await user.click(screen.getByRole("button", { name: "Select sensitive-item" }));

    const details = screen.getByRole("complementary", { name: "Gallery item details" });
    expect(within(details).getByText("sensitive-item")).toBeInTheDocument();
    expect(within(details).getByText("nai-diffusion-4-5-full")).toBeInTheDocument();
    expect(within(details).getByText("Seed 1234")).toBeInTheDocument();
    expect(within(details).getByText("NSFW 0.91")).toBeInTheDocument();
    expect(within(details).getByText("open_nsfw@onnx")).toBeInTheDocument();
    expect(within(details).getByText("original")).toBeInTheDocument();
  });

  it("uses the persisted SFW preference to blur sensitive images", async () => {
    setup({ blurSensitive: true });

    const sensitiveImage = await screen.findByAltText("sensitive-item preview");
    expect(sensitiveImage).toHaveClass("blur-md");
    expect(screen.getByAltText("safe-item preview")).not.toHaveClass("blur-md");
  });

  it("opens the selected image in a fullscreen lightbox from the preview and button", async () => {
    const { user } = setup();

    const preview = await screen.findByAltText("safe-item detail preview");
    await user.click(preview);
    expect(screen.getByRole("dialog", { name: "safe-item" })).toBeInTheDocument();
    expect(await screen.findByAltText("safe-item enlarged preview")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("dialog", { name: "safe-item" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Enlarge image" }));
    expect(screen.getByRole("dialog", { name: "safe-item" })).toBeInTheDocument();
  });

  it("filters hidden items, updates safety overrides, and exports the preferred resource", async () => {
    const { user } = setup();

    await screen.findByText("safe-item");
    await user.selectOptions(screen.getByLabelText("Safety filter"), "hidden");
    await waitForLastGalleryQuery({ safety_label: "hidden" });
    expect(await screen.findByRole("button", { name: "Select hidden-item" })).toBeInTheDocument();
    expect(screen.queryByText("safe-item")).not.toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("Safety filter"), "all");
    await user.click(await screen.findByRole("button", { name: "Select safe-item" }));
    await user.selectOptions(screen.getByLabelText("Safety override"), "hidden");
    await user.click(screen.getByRole("button", { name: "Apply safety override" }));

    expect(mocks.galleryApi.setSafetyOverride).toHaveBeenCalledWith({
      item_id: "safe-item",
      manual_safety_override: "hidden",
    });

    await user.click(screen.getByRole("button", { name: "Export selected image" }));
    expect(mocks.desktopApi.saveResourceImage).toHaveBeenCalledWith({
      resource: { id: "resource:safe-item:original", variant_id: null },
      suggested_file_name: "safe-item-original",
    });
  });

  it("sanitizes export filenames and reports command failures", async () => {
    const unsafeItem = galleryItem("artifact:job-1:sample:0", {
      source: "generation",
      label: "safe",
    });
    const { user } = setup({ items: [unsafeItem] });
    mocks.galleryApi.setSafetyOverride.mockRejectedValueOnce(new Error("override failed"));

    await screen.findByRole("button", { name: "Select artifact:job-1:sample:0" });
    await user.click(screen.getByRole("button", { name: "Export selected image" }));
    expect(mocks.desktopApi.saveResourceImage).toHaveBeenCalledWith({
      resource: { id: "resource:artifact:job-1:sample:0:original", variant_id: null },
      suggested_file_name: "artifact-job-1-sample-0-original",
    });

    await user.selectOptions(screen.getByLabelText("Safety override"), "hidden");
    await user.click(screen.getByRole("button", { name: "Apply safety override" }));
    expect(await screen.findByText("override failed")).toBeInTheDocument();

    mocks.desktopApi.saveResourceImage.mockRejectedValueOnce(new Error("export failed"));
    await user.click(screen.getByRole("button", { name: "Export selected image" }));
    expect(await screen.findByText("export failed")).toBeInTheDocument();
    expect(screen.queryByText("override failed")).not.toBeInTheDocument();
  });

  it("confirms hard delete and deletes the selected gallery item", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Select safe-item" }));
    await user.click(screen.getByRole("button", { name: "Delete selected gallery item" }));

    const dialog = screen.getByRole("dialog", { name: "Delete gallery item" });
    expect(within(dialog).getByText("safe-item")).toBeInTheDocument();

    await user.click(within(dialog).getByRole("button", { name: "Delete permanently" }));

    expect(mocks.galleryApi.deleteItems).toHaveBeenCalledWith({
      item_ids: ["safe-item"],
    });
  });

  it("keeps the selected item visible when hard delete fails", async () => {
    const { user } = setup();
    mocks.galleryApi.deleteItems.mockRejectedValueOnce(new Error("delete failed"));

    await user.click(await screen.findByRole("button", { name: "Select safe-item" }));
    await user.click(screen.getByRole("button", { name: "Delete selected gallery item" }));
    await user.click(
      within(screen.getByRole("dialog", { name: "Delete gallery item" })).getByRole("button", {
        name: "Delete permanently",
      }),
    );

    expect(
      await within(screen.getByRole("dialog", { name: "Delete gallery item" })).findByText(
        "delete failed",
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Select safe-item" })).toBeInTheDocument();
    expect(
      within(screen.getByRole("complementary", { name: "Gallery item details" })).getByText(
        "safe-item",
      ),
    ).toBeInTheDocument();
  });
});

async function waitForLastGalleryQuery(expected: Partial<GalleryQueryDto>) {
  await waitFor(() => {
    expect(mocks.galleryApi.list).toHaveBeenLastCalledWith(expect.objectContaining(expected));
  });
}
