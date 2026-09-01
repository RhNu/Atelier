import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createAtelierQueryClient } from "../app/query-client";
import { AppToastHost } from "../components/ui/AppToastHost";
import { GalleryPage } from "../features/gallery";
import type {
  CopyResourceImageRequestDto,
  GalleryItemDto,
  GalleryItemDetailDto,
  GalleryItemDetailRequestDto,
  GalleryPageDto,
  GalleryQueryDto,
  DeleteGalleryItemsRequestDto,
  DeleteGalleryItemsResponseDto,
  GetResourceImageRequestDto,
  GlobalSettingsDto,
  ResourceImageDto,
  RescanGallerySafetyRequestDto,
  RescanGallerySafetyResponseDto,
  SaveResourceImageRequestDto,
  SetGallerySafetyOverrideRequestDto,
} from "../types";

const mocks = vi.hoisted(() => ({
  galleryApi: {
    list: vi.fn<(request: GalleryQueryDto) => Promise<GalleryPageDto>>(),
    detail: vi.fn<(request: GalleryItemDetailRequestDto) => Promise<GalleryItemDetailDto>>(),
    setSafetyOverride:
      vi.fn<(request: SetGallerySafetyOverrideRequestDto) => Promise<GalleryItemDto>>(),
    rescanSafety:
      vi.fn<(request: RescanGallerySafetyRequestDto) => Promise<RescanGallerySafetyResponseDto>>(),
    deleteItems:
      vi.fn<(request: DeleteGalleryItemsRequestDto) => Promise<DeleteGalleryItemsResponseDto>>(),
  },
  resourceApi: {
    image: vi.fn<(request: GetResourceImageRequestDto) => Promise<ResourceImageDto>>(),
  },
  desktopApi: {
    copyResourceImage: vi.fn<(request: CopyResourceImageRequestDto) => Promise<void>>(),
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
      detail: (itemId: string | null) => ["gallery", "detail", itemId],
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
    convert_full_width_punctuation: false,
    gallery: {
      blur_sensitive_images: false,
    },
  },
  safety: { wd_auto_review_enabled: false },
};

function galleryItem(
  itemId: string,
  options: {
    source: GalleryItemDto["source_kind"];
    artifactKind?: string;
    label?: "safe" | "sensitive" | "hidden";
    manualOverride?: "safe" | "sensitive" | "hidden" | null;
    resourceId?: string;
    modelName?: string | null;
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
    request_seed: 1234,
    prompt: "1girl",
    negative_prompt: "lowres",
    embedded_metadata_status: "parsed",
    embedded_metadata_error: null,
    embedded_metadata_warnings: [],
    sample_index: 0,
    model_name: options.modelName === undefined ? "nai-diffusion-4-5-full" : options.modelName,
    safety: {
      scan_state: "scanned",
      risk_band: label === "sensitive" ? "high" : "low",
      auto_label: label === "hidden" ? "sensitive" : label,
      effective_label: label,
      policy_id: "anime-rating-cascade",
      policy_version: "1",
      primary: {
        model_id: "anime_dbrating",
        model_revision: "7af21db648acdeb74f5c334abda9dd7403407b3c",
        ratings: {
          general: label === "safe" ? 0.96 : 0.04,
          sensitive: 0.02,
          questionable: label === "safe" ? 0.01 : 0.2,
          explicit: label === "safe" ? 0.01 : 0.74,
        },
        fused_score: label === "safe" ? 0.04 : 0.95,
      },
      review: {
        state: "not_needed",
        model_id: null,
        model_revision: null,
        evidence: null,
        message: null,
      },
      assessed_at_ms: 1_800_000_000_001,
      message: null,
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
        modelName: null,
      }),
    ],
    total: options?.items?.length ?? 3,
    offset: 0,
    limit: 24,
  });
  mocks.galleryApi.detail.mockImplementation(async (request) => ({
    item_id: request.item_id,
    embedded_metadata_json: '{"prompt":"1girl","seed":1234}',
  }));
  mocks.galleryApi.setSafetyOverride.mockImplementation(async (request) =>
    galleryItem(request.item_id, {
      source: "generation",
      label: request.manual_safety_override ?? "safe",
      manualOverride: request.manual_safety_override ?? null,
    }),
  );
  mocks.galleryApi.rescanSafety.mockResolvedValue({
    requested: 1,
    scanned: 1,
    failed: 0,
    unavailable: 0,
  });
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
  mocks.desktopApi.copyResourceImage.mockResolvedValue();

  return {
    user: userEvent.setup(),
    ...render(
      <QueryClientProvider client={createAtelierQueryClient()}>
        <GalleryPage />
        <AppToastHost />
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

    expect(await screen.findByRole("button", { name: "Select safe-item" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Select sensitive-item" })).toBeInTheDocument();
    expect(screen.queryByText("hidden-item")).not.toBeInTheDocument();
    expect((await screen.findAllByAltText("Gallery image"))[0]).toHaveAttribute(
      "src",
      "data:image/png;base64,image-resource:safe-item:original:thumbnail",
    );

    await user.click(screen.getByRole("button", { name: "Select sensitive-item" }));

    const details = screen.getByRole("complementary", { name: "Gallery item details" });
    expect(within(details).getByText("NAI Diffusion 4.5 Full")).toBeInTheDocument();
    expect(within(details).getAllByText("1234")).toHaveLength(2);
    expect(within(details).getByText("1girl")).toBeInTheDocument();
    expect(within(details).getByText("lowres")).toBeInTheDocument();
    expect(within(details).getAllByText("Fused 0.950")).not.toHaveLength(0);
    expect(within(details).getByText(/anime_dbrating@7af21db648ac/)).toBeInTheDocument();
    expect(within(details).queryByText("nai-diffusion-4-5-full")).not.toBeInTheDocument();
    expect(within(details).queryByText("Parsed")).not.toBeInTheDocument();
    expect(within(details).queryByText("Metadata error")).not.toBeInTheDocument();
    expect(within(details).getByText("Generation metadata").closest("details")).not.toHaveAttribute(
      "open",
    );
    expect(within(details).getByText("More safety details").closest("details")).not.toHaveAttribute(
      "open",
    );
  });

  it("uses the persisted SFW preference to blur sensitive images", async () => {
    setup({ blurSensitive: true });

    const sensitiveImage = (await screen.findAllByAltText("Gallery image"))[1];
    expect(sensitiveImage).toHaveClass("blur-md");
    expect((await screen.findAllByAltText("Gallery image"))[0]).not.toHaveClass("blur-md");
  });
});

describe("Gallery safety scanning", () => {
  it("surfaces an unscanned item and requests a targeted safety rescan", async () => {
    const unscanned = galleryItem("unscanned-item", { source: "generation" });
    unscanned.safety = {
      scan_state: "unscanned",
      risk_band: null,
      auto_label: null,
      effective_label: null,
      policy_id: null,
      policy_version: null,
      primary: null,
      review: null,
      assessed_at_ms: null,
      message: null,
    };
    const { user } = setup({ items: [unscanned] });

    await screen.findByRole("button", { name: "Select unscanned-item" });
    await user.click(screen.getByText("More safety details"));
    expect(await screen.findByText("This image has not been scanned.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Rescan" }));

    expect(mocks.galleryApi.rescanSafety).toHaveBeenCalledWith({
      item_ids: ["unscanned-item"],
    });
  });
});

describe("GalleryPage commands", () => {
  it("opens the selected image in a fullscreen lightbox from the preview and button", async () => {
    const { user } = setup();

    const previewButton = await screen.findByRole("button", { name: "Enlarge image" });
    expect(previewButton).toHaveClass("outline-none", "focus-visible:ring-brand-400");
    await user.click(previewButton);
    expect(screen.getByRole("dialog", { name: "Image preview" })).toBeInTheDocument();
    expect(await screen.findByAltText("Enlarged gallery image")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("dialog", { name: "Image preview" })).not.toBeInTheDocument();

    await user.click(previewButton);
    expect(screen.getByRole("dialog", { name: "Image preview" })).toBeInTheDocument();
  });

  it("moves between enlarged images with arrow keys without wrapping at either end", async () => {
    const { user } = setup();

    await user.click(await screen.findByRole("button", { name: "Enlarge image" }));
    expect(await screen.findByAltText("Enlarged gallery image")).toHaveAttribute(
      "src",
      "data:image/png;base64,image-resource:safe-item:original:preview",
    );

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    expect(screen.getByAltText("Enlarged gallery image")).toHaveAttribute(
      "src",
      "data:image/png;base64,image-resource:safe-item:original:preview",
    );

    fireEvent.keyDown(window, { key: "ArrowRight" });
    await waitFor(() =>
      expect(screen.getByAltText("Enlarged gallery image")).toHaveAttribute(
        "src",
        "data:image/png;base64,image-resource:sensitive-item:original:preview",
      ),
    );

    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(screen.getByAltText("Enlarged gallery image")).toHaveAttribute(
      "src",
      "data:image/png;base64,image-resource:sensitive-item:original:preview",
    );

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    await waitFor(() =>
      expect(screen.getByAltText("Enlarged gallery image")).toHaveAttribute(
        "src",
        "data:image/png;base64,image-resource:safe-item:original:preview",
      ),
    );
  });

  it("filters hidden items, updates safety overrides, and exports the preferred resource", async () => {
    const { user } = setup();

    await screen.findByRole("button", { name: "Select safe-item" });
    await user.click(screen.getByLabelText("Safety filter"));
    await user.click(screen.getByRole("option", { name: "Hidden" }));
    await waitForLastGalleryQuery({ safety_label: "hidden" });
    const hiddenCard = await screen.findByRole("button", { name: "Select hidden-item" });
    expect(within(hiddenCard).getByText("Director results")).toBeInTheDocument();
    expect(screen.queryByText("safe-item")).not.toBeInTheDocument();

    await user.click(screen.getByLabelText("Safety filter"));
    await user.click(screen.getByRole("option", { name: "Safe + sensitive" }));
    await user.click(await screen.findByRole("button", { name: "Select safe-item" }));
    await user.click(screen.getByRole("button", { name: "Change safety override" }));
    const overrideDialog = screen.getByRole("dialog", { name: "Safety override" });
    await user.click(within(overrideDialog).getByLabelText("Safety override"));
    await user.click(screen.getByRole("option", { name: "Hidden" }));
    await user.click(within(overrideDialog).getByRole("button", { name: "Apply safety override" }));

    expect(mocks.galleryApi.setSafetyOverride).toHaveBeenCalledWith({
      item_id: "safe-item",
      manual_safety_override: "hidden",
    });

    await user.click(screen.getByRole("button", { name: "Export" }));
    await user.click(screen.getByRole("menuitem", { name: "PNG original" }));
    expect(mocks.desktopApi.saveResourceImage).toHaveBeenCalledWith({
      resource: { id: "resource:safe-item:original", variant_id: null },
      format: "png_original",
      suggested_file_name: "image-270115-080000",
    });
  });

  it("copies and exports explicit formats and reports command failures", async () => {
    const unsafeItem = galleryItem("artifact:job-1:sample:0", {
      source: "generation",
      label: "safe",
    });
    const { user } = setup({ items: [unsafeItem] });
    mocks.galleryApi.setSafetyOverride.mockRejectedValueOnce(new Error("override failed"));

    await screen.findByRole("button", { name: "Select artifact:job-1:sample:0" });
    await user.click(screen.getByRole("button", { name: "Copy" }));
    await user.click(screen.getByRole("menuitem", { name: "JPG" }));
    expect(mocks.desktopApi.copyResourceImage).toHaveBeenCalledWith({
      resource: { id: "resource:artifact:job-1:sample:0:original", variant_id: null },
      format: "jpeg",
    });
    expect(await screen.findByText("Image copied")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Export" }));
    await user.click(screen.getByRole("menuitem", { name: "PNG without metadata" }));
    expect(mocks.desktopApi.saveResourceImage).toHaveBeenCalledWith({
      resource: { id: "resource:artifact:job-1:sample:0:original", variant_id: null },
      format: "png_sanitized",
      suggested_file_name: "image-270115-080000",
    });

    await user.click(screen.getByRole("button", { name: "Change safety override" }));
    const overrideDialog = screen.getByRole("dialog", { name: "Safety override" });
    await user.click(within(overrideDialog).getByLabelText("Safety override"));
    await user.click(screen.getByRole("option", { name: "Hidden" }));
    await user.click(within(overrideDialog).getByRole("button", { name: "Apply safety override" }));
    expect(await screen.findByText("override failed")).toBeInTheDocument();

    mocks.desktopApi.saveResourceImage.mockRejectedValueOnce(new Error("export failed"));
    await user.click(screen.getByRole("button", { name: "Export" }));
    await user.click(screen.getByRole("menuitem", { name: "JPG" }));
    expect(await screen.findByText("export failed")).toBeInTheDocument();
    expect(screen.getByText("override failed")).toBeInTheDocument();
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

  it("selects all visible images and deletes them as one batch", async () => {
    const { user } = setup();

    const selectAll = await screen.findByRole("checkbox", { name: "Select all" });
    await user.click(screen.getByRole("checkbox", { name: "Select safe-item for batch actions" }));
    expect(selectAll).toBePartiallyChecked();
    expect(screen.getByRole("button", { name: "Delete selected (1)" })).toBeEnabled();

    await user.click(selectAll);

    expect(
      screen.getByRole("checkbox", { name: "Select safe-item for batch actions" }),
    ).toBeChecked();
    expect(
      screen.getByRole("checkbox", { name: "Select sensitive-item for batch actions" }),
    ).toBeChecked();

    await user.click(screen.getByRole("button", { name: "Delete selected (2)" }));
    const dialog = screen.getByRole("dialog", { name: "Delete selected gallery items" });
    expect(within(dialog).getByText("Delete 2 selected gallery items permanently?")).toBeVisible();
    await user.click(within(dialog).getByRole("button", { name: "Delete permanently" }));

    expect(mocks.galleryApi.deleteItems).toHaveBeenCalledWith({
      item_ids: ["safe-item", "sensitive-item"],
    });
    await waitFor(() => expect(selectAll).not.toBeChecked());
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
        "Generated images",
      ),
    ).toBeInTheDocument();
  });
});

async function waitForLastGalleryQuery(expected: Partial<GalleryQueryDto>) {
  await waitFor(() => {
    expect(mocks.galleryApi.list).toHaveBeenLastCalledWith(expect.objectContaining(expected));
  });
}
