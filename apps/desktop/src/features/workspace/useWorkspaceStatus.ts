import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

import { applyLanguagePreference } from "@/i18n";
import {
  AtelierCommandError,
  clearWorkspaceScopedQueryCache,
  desktopApi,
  globalSettingsApi,
  queryKeys,
  workspaceApi,
} from "@/platform/atelier";
import type {
  AppBootstrapDto,
  FrontendLanguageDto,
  WorkspaceRestoreFailureDto,
  WorkspaceStatusDto,
} from "@/types";

import { resetGenerationEventState } from "../generation/state/generation-event-store";

export type WorkspaceStatusView = {
  workspaceStatus: WorkspaceStatusDto | null;
  workspacePending: boolean;
  workspaceErrorCode: string | undefined;
  workspaceErrorMessage: string | undefined;
  restoreFailure: WorkspaceRestoreFailureDto | null;
  openWorkspace: () => void;
  retryWorkspaceRestore: () => void;
  closeWorkspace: () => void;
  openingWorkspace: boolean;
  closingWorkspace: boolean;
  language: FrontendLanguageDto;
  languagePending: boolean;
  languageErrorMessage: string | undefined;
  changeLanguage: (language: FrontendLanguageDto) => void;
};

function getCommandError(error: unknown): AtelierCommandError | null {
  return error instanceof AtelierCommandError ? error : null;
}

export function useWorkspaceStatus(): WorkspaceStatusView {
  const queryClient = useQueryClient();

  const bootstrapQuery = useQuery({
    queryKey: queryKeys.app.bootstrap(),
    queryFn: () => workspaceApi.bootstrap(),
    retry: 1,
  });

  const openMutation = useMutation({
    mutationFn: async () => {
      const root = await desktopApi.pickWorkspaceDirectory();

      if (!root) {
        return null;
      }

      const status = await workspaceApi.open({ root });
      const globalSettings = await globalSettingsApi.get();
      return { status, globalSettings };
    },
    onSuccess: async (result) => {
      if (!result) {
        return;
      }

      resetGenerationEventState();
      await clearWorkspaceScopedQueryCache(queryClient);
      queryClient.setQueryData(queryKeys.app.globalSettings(), result.globalSettings);
      queryClient.setQueryData<AppBootstrapDto>(queryKeys.app.bootstrap(), {
        global_settings: result.globalSettings,
        workspace: result.status,
        restore_failure: null,
      });
    },
  });

  const closeMutation = useMutation({
    mutationFn: () => workspaceApi.close(),
    onSuccess: async () => {
      resetGenerationEventState();
      await clearWorkspaceScopedQueryCache(queryClient);
      queryClient.setQueryData<AppBootstrapDto | undefined>(queryKeys.app.bootstrap(), (current) =>
        current ? { ...current, workspace: null, restore_failure: null } : current,
      );
    },
  });

  const languageMutation = useMutation({
    mutationFn: async (language: FrontendLanguageDto) => {
      const current = bootstrapQuery.data?.global_settings;
      if (!current) throw new Error("Global settings are not loaded.");
      return globalSettingsApi.update({
        frontend: { ...current.frontend, language },
      });
    },
    onSuccess: async (settings) => {
      queryClient.setQueryData(queryKeys.app.globalSettings(), settings);
      queryClient.setQueryData<AppBootstrapDto | undefined>(queryKeys.app.bootstrap(), (current) =>
        current ? { ...current, global_settings: settings } : current,
      );
      await applyLanguagePreference(settings.frontend.language);
    },
  });

  const language = bootstrapQuery.data?.global_settings.frontend.language ?? "system";
  useEffect(() => {
    void applyLanguagePreference(language);
  }, [language]);

  const error = getCommandError(bootstrapQuery.error ?? openMutation.error ?? closeMutation.error);
  const languageError = getCommandError(languageMutation.error);

  return {
    workspaceStatus: bootstrapQuery.data?.workspace ?? null,
    workspacePending: bootstrapQuery.isPending || openMutation.isPending || closeMutation.isPending,
    workspaceErrorCode: error?.code,
    workspaceErrorMessage: error?.message,
    restoreFailure: bootstrapQuery.data?.restore_failure ?? null,
    openWorkspace: () => openMutation.mutate(),
    retryWorkspaceRestore: () => {
      void bootstrapQuery.refetch();
    },
    closeWorkspace: () => closeMutation.mutate(),
    openingWorkspace: openMutation.isPending,
    closingWorkspace: closeMutation.isPending,
    language,
    languagePending: languageMutation.isPending,
    languageErrorMessage:
      languageError?.message ??
      (languageMutation.error instanceof Error ? languageMutation.error.message : undefined),
    changeLanguage: (nextLanguage) => languageMutation.mutate(nextLanguage),
  };
}
