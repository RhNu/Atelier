import type {
  ExploreItemRefDto,
  ExploreMediaRequestDto,
  ExplorePageDto,
  ExplorePostDetailDto,
  ExploreSearchRequestDto,
  ExploreSourceDescriptorDto,
  ResourceImageDto,
} from "@/types";

import { atelierCommands } from "./commands";
import { invokeAtelierCommand } from "./tauri-client";

export const exploreApi = {
  sources: () =>
    invokeAtelierCommand<ExploreSourceDescriptorDto[]>(atelierCommands.listExploreSources),
  search: (request: ExploreSearchRequestDto) =>
    invokeAtelierCommand<ExplorePageDto>(atelierCommands.searchExplorePosts, { request }),
  detail: (item: ExploreItemRefDto) =>
    invokeAtelierCommand<ExplorePostDetailDto>(atelierCommands.getExplorePostDetail, { item }),
  media: (request: ExploreMediaRequestDto) =>
    invokeAtelierCommand<ResourceImageDto>(atelierCommands.getExploreMedia, { request }),
};
