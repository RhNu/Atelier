/* eslint-disable max-lines */
import { Channel } from "@tauri-apps/api/core";

import type {
  ApiKeyRecordDto,
  AppendLexiconEntitiesRequestDto,
  AppBootstrapDto,
  CloseWorkspaceResponseDto,
  CompilePromptRequestDto,
  CompileGenerationPromptRequestDto,
  CompiledGenerationPromptDto,
  CompiledPromptDto,
  CopyResourceImageRequestDto,
  CreateApiKeyRequestDto,
  DanbooruAccountDto,
  DanbooruMediaRequestDto,
  DanbooruPostDetailDto,
  DanbooruPostDetailRequestDto,
  DanbooruPostPageDto,
  DanbooruSearchRequestDto,
  DeleteApiKeyRequestDto,
  DeleteApiKeyResponseDto,
  DeleteGalleryItemsRequestDto,
  DeleteGalleryItemsResponseDto,
  DeleteRunHistoryItemsRequestDto,
  DeleteRunHistoryItemsResponseDto,
  DeleteGenerationHistoryBatchesRequestDto,
  DeleteGenerationHistoryBatchesResponseDto,
  DeletePromptChunkRequestDto,
  DeletePromptChunkResponseDto,
  DeletePromptPresetRequestDto,
  DeletePromptPresetResponseDto,
  EnsureVibeEncodingRequestDto,
  EnsuredVibeEncodingDto,
  EventsSinceRequestDto,
  AppEventPageDto,
  ExportVibeDocumentRequestDto,
  GalleryImageReferenceDto,
  GalleryImageReferenceRequestDto,
  GalleryItemDetailDto,
  GalleryItemDetailRequestDto,
  GalleryItemDto,
  GalleryPageDto,
  GalleryQueryDto,
  GlobalSettingsDto,
  GenerationDraftDto,
  GenerationStatusDto,
  GenerationAnlasEstimateDto,
  GenerationEstimateRequestDto,
  GenerationStatusQueryDto,
  GenerationHistoryBatchDetailDto,
  GenerationHistoryBatchRequestDto,
  GenerationHistoryPageDto,
  GenerationHistoryQueryDto,
  GetPromptChunkRequestDto,
  GetResourceImageRequestDto,
  GetVibeDocumentRequestDto,
  ImageResourceKindDto,
  ImageAnalysisModelInstallProgressDto,
  ImageAnalysisModelRequestDto,
  ImageAnalysisModelStatusDto,
  ImportImageResourceResponseDto,
  ImportedVibeDocumentsDto,
  ListVibeDocumentsRequestDto,
  ListPromptChunksRequestDto,
  ListPromptPresetsRequestDto,
  OpenWorkspaceRequestDto,
  ProbeApiKeyRequestDto,
  PromptChunkDto,
  PromptChunkPageDto,
  LexiconBootstrapDto,
  LexiconCompleteRequestDto,
  LexiconEntityDetailDto,
  LexiconEntityRequestDto,
  LexiconSearchItemDto,
  LexiconSearchPageDto,
  LexiconSearchRequestDto,
  PromptPresetDto,
  PromptPresetPageDto,
  QueueDirectiveDto,
  RerunGenerationHistoryItemRequestDto,
  RerunGenerationHistoryItemResponseDto,
  RerunGenerationHistoryBatchRequestDto,
  RerunGenerationHistoryBatchResponseDto,
  ResourceImageDto,
  ReleaseImportedImageResourcesRequestDto,
  ReleaseImportedImageResourcesResponseDto,
  SaveResourceImageRequestDto,
  SaveDanbooruAccountRequestDto,
  SaveResourceImagesZipRequestDto,
  SaveGenerationDraftRequestDto,
  ResetWorkspaceSettingsResponseDto,
  RunDirectorToolRequestDto,
  DirectorToolResultDto,
  RunGenerationJobRequestDto,
  RunHistoryPageDto,
  RunHistoryQueryDto,
  SetActiveApiKeyRequestDto,
  SetGallerySafetyOverrideRequestDto,
  RescanGallerySafetyRequestDto,
  RescanGallerySafetyResponseDto,
  RenameVibeDocumentRequestDto,
  SetVibeDocumentHiddenRequestDto,
  SubmitGenerationRequestDto,
  SubmitGenerationBatchRequestDto,
  SubscriptionSummaryDto,
  UpdateApiKeyRequestDto,
  UpdateGlobalSettingsRequestDto,
  UpdateWorkspaceSettingsRequestDto,
  UpsertPromptChunkRequestDto,
  UpsertPromptPresetRequestDto,
  VibeDocumentEntryDto,
  VibeDocumentPageDto,
  WorkspaceSettingsDto,
  WorkspaceStatusDto,
} from "@/types";

import { atelierCommands } from "./commands";
import { invokeAtelierCommand } from "./tauri-client";

export type DesktopPathsDto = {
  app_data_dir: string;
  app_config_dir: string;
  app_cache_dir: string;
  suggested_workspace_dir: string;
  resource_dir: string | null;
};

export type PickFilesOptionsDto = {
  extensions: string[];
};

export type ClipboardImageDto = {
  imageBase64: string;
  mimeType: string;
};

export const desktopApi = {
  paths: () => invokeAtelierCommand<DesktopPathsDto>(atelierCommands.desktopPaths),
  readClipboardImage: () =>
    invokeAtelierCommand<ClipboardImageDto>(atelierCommands.readClipboardImage),
  importClipboardImageResource: (kind: ImageResourceKindDto) =>
    invokeAtelierCommand<ImportImageResourceResponseDto>(
      atelierCommands.importClipboardImageResource,
      { kind },
    ),
  pickWorkspaceDirectory: () =>
    invokeAtelierCommand<string | null>(atelierCommands.pickWorkspaceDirectory),
  pickExportDirectory: () =>
    invokeAtelierCommand<string | null>(atelierCommands.pickExportDirectory),
  pickAndImportImageResources: (kind: ImageResourceKindDto, options: PickFilesOptionsDto) =>
    invokeAtelierCommand<ImportImageResourceResponseDto[]>(
      atelierCommands.pickAndImportImageResources,
      { kind, options },
    ),
  pickAndImportVibeDocuments: (options: PickFilesOptionsDto) =>
    invokeAtelierCommand<ImportedVibeDocumentsDto>(atelierCommands.pickAndImportVibeDocuments, {
      options,
    }),
  pickAndImportEmbeddedPngVibeDocuments: (options: PickFilesOptionsDto) =>
    invokeAtelierCommand<ImportedVibeDocumentsDto>(
      atelierCommands.pickAndImportEmbeddedPngVibeDocuments,
      { options },
    ),
  saveResourceImage: (request: SaveResourceImageRequestDto) =>
    invokeAtelierCommand<{ path: string } | null>(atelierCommands.saveResourceImage, { request }),
  copyResourceImage: (request: CopyResourceImageRequestDto) =>
    invokeAtelierCommand<void>(atelierCommands.copyResourceImage, { request }),
  saveResourceImagesZip: (request: SaveResourceImagesZipRequestDto) =>
    invokeAtelierCommand<{ path: string; exported: number } | null>(
      atelierCommands.saveResourceImagesZip,
      { request },
    ),
  openPath: (path: string) => invokeAtelierCommand<void>(atelierCommands.openPath, { path }),
  revealPath: (path: string) => invokeAtelierCommand<void>(atelierCommands.revealPath, { path }),
  copyText: (text: string) =>
    invokeAtelierCommand<void>(atelierCommands.copyTextToClipboard, { text }),
  openExternalUrl: (url: string) =>
    invokeAtelierCommand<void>(atelierCommands.openExternalUrl, { url }),
};

export const workspaceApi = {
  bootstrap: () => invokeAtelierCommand<AppBootstrapDto>(atelierCommands.bootstrapApp),
  open: (request: OpenWorkspaceRequestDto) =>
    invokeAtelierCommand<WorkspaceStatusDto>(atelierCommands.openWorkspace, { request }),
  status: () => invokeAtelierCommand<WorkspaceStatusDto | null>(atelierCommands.workspaceStatus),
  close: () => invokeAtelierCommand<CloseWorkspaceResponseDto>(atelierCommands.closeWorkspace),
};

export const globalSettingsApi = {
  get: () => invokeAtelierCommand<GlobalSettingsDto>(atelierCommands.getGlobalSettings),
  update: (request: UpdateGlobalSettingsRequestDto) =>
    invokeAtelierCommand<GlobalSettingsDto>(atelierCommands.updateGlobalSettings, { request }),
};

export const danbooruApi = {
  account: () => invokeAtelierCommand<DanbooruAccountDto>(atelierCommands.getDanbooruAccount),
  saveAccount: (request: SaveDanbooruAccountRequestDto) =>
    invokeAtelierCommand<DanbooruAccountDto>(atelierCommands.saveDanbooruAccount, { request }),
  probeAccount: () =>
    invokeAtelierCommand<DanbooruAccountDto>(atelierCommands.probeDanbooruAccount),
  deleteAccount: () =>
    invokeAtelierCommand<DanbooruAccountDto>(atelierCommands.deleteDanbooruAccount),
  search: (request: DanbooruSearchRequestDto) =>
    invokeAtelierCommand<DanbooruPostPageDto>(atelierCommands.searchDanbooruPosts, { request }),
  detail: (request: DanbooruPostDetailRequestDto) =>
    invokeAtelierCommand<DanbooruPostDetailDto>(atelierCommands.getDanbooruPostDetail, {
      request,
    }),
  media: (request: DanbooruMediaRequestDto) =>
    invokeAtelierCommand<ResourceImageDto>(atelierCommands.getDanbooruMedia, { request }),
};

export const imageAnalysisApi = {
  statuses: () =>
    invokeAtelierCommand<ImageAnalysisModelStatusDto[]>(
      atelierCommands.getImageAnalysisModelStatus,
    ),
  install: (
    request: ImageAnalysisModelRequestDto,
    onProgress: (progress: ImageAnalysisModelInstallProgressDto) => void,
  ) => {
    const channel = new Channel<ImageAnalysisModelInstallProgressDto>(onProgress);
    return invokeAtelierCommand<ImageAnalysisModelStatusDto>(
      atelierCommands.installImageAnalysisModel,
      { request, onProgress: channel },
    );
  },
  cancelInstall: (request: ImageAnalysisModelRequestDto) =>
    invokeAtelierCommand<void>(atelierCommands.cancelImageAnalysisModelInstall, { request }),
  delete: (request: ImageAnalysisModelRequestDto) =>
    invokeAtelierCommand<void>(atelierCommands.deleteImageAnalysisModel, { request }),
};

export const accountApi = {
  create: (request: CreateApiKeyRequestDto) =>
    invokeAtelierCommand<ApiKeyRecordDto>(atelierCommands.createApiKey, { request }),
  update: (request: UpdateApiKeyRequestDto) =>
    invokeAtelierCommand<ApiKeyRecordDto>(atelierCommands.updateApiKey, { request }),
  delete: (request: DeleteApiKeyRequestDto) =>
    invokeAtelierCommand<DeleteApiKeyResponseDto>(atelierCommands.deleteApiKey, { request }),
  list: () => invokeAtelierCommand<ApiKeyRecordDto[]>(atelierCommands.listApiKeys),
  setActive: (request: SetActiveApiKeyRequestDto) =>
    invokeAtelierCommand<void>(atelierCommands.setActiveApiKey, { request }),
  probe: (request: ProbeApiKeyRequestDto) =>
    invokeAtelierCommand<SubscriptionSummaryDto>(atelierCommands.probeApiKey, { request }),
  probeActive: () =>
    invokeAtelierCommand<SubscriptionSummaryDto>(atelierCommands.probeActiveApiKey),
};

export const promptApi = {
  upsertChunk: (request: UpsertPromptChunkRequestDto) =>
    invokeAtelierCommand<PromptChunkDto>(atelierCommands.upsertPromptChunk, { request }),
  getChunk: (request: GetPromptChunkRequestDto) =>
    invokeAtelierCommand<PromptChunkDto>(atelierCommands.getPromptChunk, { request }),
  listChunks: (request: ListPromptChunksRequestDto) =>
    invokeAtelierCommand<PromptChunkPageDto>(atelierCommands.listPromptChunks, { request }),
  deleteChunk: (request: DeletePromptChunkRequestDto) =>
    invokeAtelierCommand<DeletePromptChunkResponseDto>(atelierCommands.deletePromptChunk, {
      request,
    }),
  upsertPreset: (request: UpsertPromptPresetRequestDto) =>
    invokeAtelierCommand<PromptPresetDto>(atelierCommands.upsertPromptPreset, { request }),
  listPresets: (request: ListPromptPresetsRequestDto) =>
    invokeAtelierCommand<PromptPresetPageDto>(atelierCommands.listPromptPresets, { request }),
  deletePreset: (request: DeletePromptPresetRequestDto) =>
    invokeAtelierCommand<DeletePromptPresetResponseDto>(atelierCommands.deletePromptPreset, {
      request,
    }),
  compilePreview: (request: CompilePromptRequestDto) =>
    invokeAtelierCommand<CompiledPromptDto>(atelierCommands.compilePromptPreview, { request }),
  compileGenerationPreview: (request: CompileGenerationPromptRequestDto) =>
    invokeAtelierCommand<CompiledGenerationPromptDto>(
      atelierCommands.compileGenerationPromptPreview,
      { request },
    ),
};

export const lexiconApi = {
  bootstrap: () => invokeAtelierCommand<LexiconBootstrapDto>(atelierCommands.lexiconBootstrap),
  complete: (request: LexiconCompleteRequestDto) =>
    invokeAtelierCommand<LexiconSearchItemDto[]>(atelierCommands.lexiconComplete, { request }),
  search: (request: LexiconSearchRequestDto) =>
    invokeAtelierCommand<LexiconSearchPageDto>(atelierCommands.lexiconSearch, { request }),
  entity: (request: LexiconEntityRequestDto) =>
    invokeAtelierCommand<LexiconEntityDetailDto>(atelierCommands.lexiconEntity, { request }),
};

export const resourceApi = {
  image: (request: GetResourceImageRequestDto) =>
    invokeAtelierCommand<ResourceImageDto>(atelierCommands.getResourceImage, { request }),
  releaseImportedImages: (request: ReleaseImportedImageResourcesRequestDto) =>
    invokeAtelierCommand<ReleaseImportedImageResourcesResponseDto>(
      atelierCommands.releaseImportedImageResources,
      { request },
    ),
};

export const settingsApi = {
  get: () => invokeAtelierCommand<WorkspaceSettingsDto>(atelierCommands.getWorkspaceSettings),
  update: (request: UpdateWorkspaceSettingsRequestDto) =>
    invokeAtelierCommand<WorkspaceSettingsDto>(atelierCommands.updateWorkspaceSettings, {
      request,
    }),
  reset: () =>
    invokeAtelierCommand<ResetWorkspaceSettingsResponseDto>(atelierCommands.resetWorkspaceSettings),
};

export const generationApi = {
  getDraft: () =>
    invokeAtelierCommand<GenerationDraftDto | null>(atelierCommands.getGenerationDraft),
  saveDraft: (request: SaveGenerationDraftRequestDto) =>
    invokeAtelierCommand<GenerationDraftDto>(atelierCommands.saveGenerationDraft, { request }),
  clearDraft: () => invokeAtelierCommand<void>(atelierCommands.clearGenerationDraft),
  appendLexiconEntities: (request: AppendLexiconEntitiesRequestDto) =>
    invokeAtelierCommand<GenerationDraftDto>(
      atelierCommands.appendLexiconEntitiesToGenerationDraft,
      { request },
    ),
  submit: (request: SubmitGenerationRequestDto) =>
    invokeAtelierCommand<QueueDirectiveDto>(atelierCommands.submitGeneration, { request }),
  submitBatch: (request: SubmitGenerationBatchRequestDto) =>
    invokeAtelierCommand<QueueDirectiveDto>(atelierCommands.submitGenerationBatch, { request }),
  estimate: (request: GenerationEstimateRequestDto) =>
    invokeAtelierCommand<GenerationAnlasEstimateDto>(atelierCommands.estimateGeneration, {
      request,
    }),
  runJob: (request: RunGenerationJobRequestDto) =>
    invokeAtelierCommand<QueueDirectiveDto>(atelierCommands.runGenerationJob, { request }),
  pause: () => invokeAtelierCommand<QueueDirectiveDto>(atelierCommands.pauseGenerationQueue),
  resume: () => invokeAtelierCommand<QueueDirectiveDto>(atelierCommands.resumeGenerationQueue),
  stop: () => invokeAtelierCommand<QueueDirectiveDto>(atelierCommands.stopGenerationQueue),
  delayElapsed: () =>
    invokeAtelierCommand<QueueDirectiveDto>(atelierCommands.generationDelayElapsed),
  status: (request: GenerationStatusQueryDto) =>
    invokeAtelierCommand<GenerationStatusDto>(atelierCommands.generationStatus, { request }),
};

export const historyApi = {
  list: (request: RunHistoryQueryDto) =>
    invokeAtelierCommand<RunHistoryPageDto>(atelierCommands.queryRunHistory, { request }),
  deleteItems: (request: DeleteRunHistoryItemsRequestDto) =>
    invokeAtelierCommand<DeleteRunHistoryItemsResponseDto>(atelierCommands.deleteRunHistoryItems, {
      request,
    }),
  rerunGeneration: (request: RerunGenerationHistoryItemRequestDto) =>
    invokeAtelierCommand<RerunGenerationHistoryItemResponseDto>(
      atelierCommands.rerunGenerationHistoryItem,
      { request },
    ),
  listGenerationBatches: (request: GenerationHistoryQueryDto) =>
    invokeAtelierCommand<GenerationHistoryPageDto>(atelierCommands.queryGenerationHistory, {
      request,
    }),
  getGenerationBatch: (request: GenerationHistoryBatchRequestDto) =>
    invokeAtelierCommand<GenerationHistoryBatchDetailDto>(
      atelierCommands.getGenerationHistoryBatch,
      {
        request,
      },
    ),
  deleteGenerationBatches: (request: DeleteGenerationHistoryBatchesRequestDto) =>
    invokeAtelierCommand<DeleteGenerationHistoryBatchesResponseDto>(
      atelierCommands.deleteGenerationHistoryBatches,
      { request },
    ),
  rerunGenerationBatch: (request: RerunGenerationHistoryBatchRequestDto) =>
    invokeAtelierCommand<RerunGenerationHistoryBatchResponseDto>(
      atelierCommands.rerunGenerationHistoryBatch,
      { request },
    ),
};

export const directorApi = {
  runTool: (request: RunDirectorToolRequestDto) =>
    invokeAtelierCommand<DirectorToolResultDto>(atelierCommands.runDirectorTool, { request }),
};

export const vibeApi = {
  listDocuments: (request: ListVibeDocumentsRequestDto) =>
    invokeAtelierCommand<VibeDocumentPageDto>(atelierCommands.listVibeDocuments, { request }),
  getDocument: (request: GetVibeDocumentRequestDto) =>
    invokeAtelierCommand<VibeDocumentEntryDto>(atelierCommands.getVibeDocument, { request }),
  renameDocument: (request: RenameVibeDocumentRequestDto) =>
    invokeAtelierCommand<VibeDocumentEntryDto>(atelierCommands.renameVibeDocument, { request }),
  setDocumentHidden: (request: SetVibeDocumentHiddenRequestDto) =>
    invokeAtelierCommand<VibeDocumentEntryDto>(atelierCommands.setVibeDocumentHidden, {
      request,
    }),
  ensureEncoding: (request: EnsureVibeEncodingRequestDto) =>
    invokeAtelierCommand<EnsuredVibeEncodingDto>(atelierCommands.ensureVibeEncoding, { request }),
  saveDocument: (request: ExportVibeDocumentRequestDto) =>
    invokeAtelierCommand<{ path: string } | null>(atelierCommands.saveVibeDocument, { request }),
};

export const galleryApi = {
  list: (request: GalleryQueryDto) =>
    invokeAtelierCommand<GalleryPageDto>(atelierCommands.queryGallery, { request }),
  detail: (request: GalleryItemDetailRequestDto) =>
    invokeAtelierCommand<GalleryItemDetailDto>(atelierCommands.getGalleryItemDetail, { request }),
  setSafetyOverride: (request: SetGallerySafetyOverrideRequestDto) =>
    invokeAtelierCommand<GalleryItemDto>(atelierCommands.setGallerySafetyOverride, { request }),
  rescanSafety: (request: RescanGallerySafetyRequestDto) =>
    invokeAtelierCommand<RescanGallerySafetyResponseDto>(atelierCommands.rescanGallerySafety, {
      request,
    }),
  deleteItems: (request: DeleteGalleryItemsRequestDto) =>
    invokeAtelierCommand<DeleteGalleryItemsResponseDto>(atelierCommands.deleteGalleryItems, {
      request,
    }),
  imageReference: (request: GalleryImageReferenceRequestDto) =>
    invokeAtelierCommand<GalleryImageReferenceDto>(atelierCommands.galleryImageReference, {
      request,
    }),
};

export const eventsApi = {
  since: (request: EventsSinceRequestDto) =>
    invokeAtelierCommand<AppEventPageDto>(atelierCommands.eventsSince, { request }),
};
