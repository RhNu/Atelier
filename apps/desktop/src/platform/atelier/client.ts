import type {
  ApiKeyRecordDto,
  CloseWorkspaceResponseDto,
  CompilePromptRequestDto,
  CompileGenerationPromptRequestDto,
  CompiledGenerationPromptDto,
  CompiledPromptDto,
  CreateApiKeyRequestDto,
  DeleteApiKeyRequestDto,
  DeleteApiKeyResponseDto,
  DeleteGalleryItemsRequestDto,
  DeleteGalleryItemsResponseDto,
  DeleteRunHistoryItemsRequestDto,
  DeleteRunHistoryItemsResponseDto,
  DeletePromptChunkRequestDto,
  DeletePromptChunkResponseDto,
  EnsureVibeEncodingRequestDto,
  EnsuredVibeEncodingDto,
  EventsSinceRequestDto,
  AppEventPageDto,
  ExportVibeDocumentRequestDto,
  GalleryImageReferenceDto,
  GalleryImageReferenceRequestDto,
  GalleryItemDto,
  GalleryPageDto,
  GalleryQueryDto,
  GenerationStatusDto,
  GenerationAnlasEstimateDto,
  GenerationEstimateRequestDto,
  GenerationStatusQueryDto,
  GetPromptChunkRequestDto,
  GetResourceImageRequestDto,
  GetVibeDocumentRequestDto,
  ImageResourceKindDto,
  ImportImageResourceResponseDto,
  ImportedVibeDocumentsDto,
  ListVibeDocumentsRequestDto,
  ListPromptChunksRequestDto,
  OpenWorkspaceRequestDto,
  ProbeApiKeyRequestDto,
  PromptChunkDto,
  PromptChunkPageDto,
  PromptLexiconCatalogDto,
  PromptLexiconListQueryDto,
  PromptLexiconPageDto,
  PromptLexiconSearchQueryDto,
  QueueDirectiveDto,
  RerunGenerationHistoryItemRequestDto,
  RerunGenerationHistoryItemResponseDto,
  ResourceImageDto,
  SaveResourceImageRequestDto,
  ResetWorkspaceSettingsResponseDto,
  RunDirectorToolRequestDto,
  DirectorToolResultDto,
  RunGenerationJobRequestDto,
  RunHistoryPageDto,
  RunHistoryQueryDto,
  SetActiveApiKeyRequestDto,
  SetGallerySafetyOverrideRequestDto,
  SubmitGenerationRequestDto,
  SubmitGenerationBatchRequestDto,
  SubscriptionSummaryDto,
  UpdateApiKeyRequestDto,
  UpdateWorkspaceSettingsRequestDto,
  UpsertPromptChunkRequestDto,
  VibeDocumentEntryDto,
  VibeDocumentPageDto,
  WorkspaceSettingsDto,
  WorkspaceStatusDto,
} from "../../types";
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

export const desktopApi = {
  paths: () => invokeAtelierCommand<DesktopPathsDto>(atelierCommands.desktopPaths),
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
  openPath: (path: string) => invokeAtelierCommand<void>(atelierCommands.openPath, { path }),
  revealPath: (path: string) => invokeAtelierCommand<void>(atelierCommands.revealPath, { path }),
};

export const workspaceApi = {
  open: (request: OpenWorkspaceRequestDto) =>
    invokeAtelierCommand<WorkspaceStatusDto>(atelierCommands.openWorkspace, { request }),
  status: () => invokeAtelierCommand<WorkspaceStatusDto>(atelierCommands.workspaceStatus),
  close: () => invokeAtelierCommand<CloseWorkspaceResponseDto>(atelierCommands.closeWorkspace),
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
  compilePreview: (request: CompilePromptRequestDto) =>
    invokeAtelierCommand<CompiledPromptDto>(atelierCommands.compilePromptPreview, { request }),
  compileGenerationPreview: (request: CompileGenerationPromptRequestDto) =>
    invokeAtelierCommand<CompiledGenerationPromptDto>(
      atelierCommands.compileGenerationPromptPreview,
      { request },
    ),
  lexiconCatalog: () =>
    invokeAtelierCommand<PromptLexiconCatalogDto>(atelierCommands.promptLexiconCatalog),
  lexiconList: (request: PromptLexiconListQueryDto) =>
    invokeAtelierCommand<PromptLexiconPageDto>(atelierCommands.promptLexiconList, { request }),
  lexiconSearch: (request: PromptLexiconSearchQueryDto) =>
    invokeAtelierCommand<PromptLexiconPageDto>(atelierCommands.promptLexiconSearch, { request }),
};

export const resourceApi = {
  image: (request: GetResourceImageRequestDto) =>
    invokeAtelierCommand<ResourceImageDto>(atelierCommands.getResourceImage, { request }),
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
  ensureEncoding: (request: EnsureVibeEncodingRequestDto) =>
    invokeAtelierCommand<EnsuredVibeEncodingDto>(atelierCommands.ensureVibeEncoding, { request }),
  saveDocument: (request: ExportVibeDocumentRequestDto) =>
    invokeAtelierCommand<{ path: string } | null>(atelierCommands.saveVibeDocument, { request }),
};

export const galleryApi = {
  list: (request: GalleryQueryDto) =>
    invokeAtelierCommand<GalleryPageDto>(atelierCommands.queryGallery, { request }),
  setSafetyOverride: (request: SetGallerySafetyOverrideRequestDto) =>
    invokeAtelierCommand<GalleryItemDto>(atelierCommands.setGallerySafetyOverride, { request }),
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
