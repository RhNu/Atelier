import {
  pickedCompletion,
  type Completion,
  type CompletionContext,
  type CompletionResult,
  type CompletionSource,
} from "@codemirror/autocomplete";
import type { QueryClient } from "@tanstack/react-query";

import { frontendLogger } from "@/app/logger";
import type { PromptChunkDto, PromptLexiconEntryDto } from "@/types";

import { fetchPromptCompletionChunks, fetchPromptCompletionTags } from "./completion-data";
import {
  buildPromptCompletionEdit,
  getPromptCompletionContext,
  type PromptCompletionContext,
  type PromptCompletionItem,
} from "./completion-model";

export type PromptCompletionMessages = {
  reusableChunk: string;
  promptChunk: string;
};

const FUNCTION_ITEM: PromptCompletionItem = {
  kind: "function",
  id: "function:chunk",
  label: "chunk",
  value: "chunk",
  detail: null,
  rank: "function",
};
const VISIBLE_CHUNK_LIMIT = 20;

export function createNaiPromptCompletion(
  queryClient: QueryClient,
  messages: PromptCompletionMessages,
): CompletionSource {
  return async (context) => completePrompt(context, queryClient, messages);
}

async function completePrompt(
  context: CompletionContext,
  queryClient: QueryClient,
  messages: PromptCompletionMessages,
): Promise<CompletionResult | null> {
  context.addEventListener("abort", () => undefined, { onDocChange: true });
  const source = context.state.doc.toString();
  const promptContext = getPromptCompletionContext(source, context.pos, context.explicit);
  frontendLogger.debug("Prompt completion started", {
    mode: promptContext.mode,
    explicit: context.explicit,
  });

  if (promptContext.mode === "function") {
    const options = functionItems(promptContext, messages).map((item) =>
      completionForItem(item, promptContext),
    );
    return options.length > 0
      ? {
          from: promptContext.filterStart,
          to: promptContext.filterEnd,
          options,
          validFor: /^[A-Za-z0-9_-]*$/u,
        }
      : null;
  }

  if (
    !context.explicit &&
    promptContext.mode === "tag" &&
    promptContext.query.trim().length === 0
  ) {
    return null;
  }

  const [chunks, tags] = await Promise.all([
    shouldFetchChunks(promptContext)
      ? recover(fetchPromptCompletionChunks(queryClient))
      : Promise.resolve([]),
    shouldFetchTags(promptContext)
      ? recover(fetchPromptCompletionTags(queryClient, promptContext.query.trim()))
      : Promise.resolve([]),
  ]);
  if (context.aborted) return null;

  const items = completionItems(promptContext, chunks, tags, messages);
  frontendLogger.debug("Prompt completion completed", {
    mode: promptContext.mode,
    itemCount: items.length,
  });
  if (items.length === 0) return null;
  return {
    from: promptContext.filterStart,
    to: promptContext.filterEnd,
    options: items.map((item) => completionForItem(item, promptContext)),
    filter: false,
  };
}

function completionItems(
  context: PromptCompletionContext,
  chunks: PromptChunkDto[],
  tags: PromptLexiconEntryDto[],
  messages: PromptCompletionMessages,
): PromptCompletionItem[] {
  if (context.mode === "chunk") return chunkItems(chunks, context.query, messages);
  const functions = context.manual ? functionItems(context, messages) : [];
  const chunkResults = context.manual ? chunkItems(chunks, context.query, messages) : [];
  return [...functions, ...chunkResults, ...tags.map(tagItem)];
}

function functionItems(
  context: PromptCompletionContext,
  messages: PromptCompletionMessages,
): PromptCompletionItem[] {
  const query = normalize(context.query);
  if (query && !FUNCTION_ITEM.label.includes(query)) return [];
  return [{ ...FUNCTION_ITEM, detail: messages.reusableChunk }];
}

function chunkItems(
  chunks: PromptChunkDto[],
  query: string,
  messages: PromptCompletionMessages,
): PromptCompletionItem[] {
  const normalizedQuery = normalize(query);
  return chunks
    .filter((chunk) => !normalizedQuery || chunkMatches(chunk, normalizedQuery))
    .slice(0, VISIBLE_CHUNK_LIMIT)
    .map((chunk) => ({
      kind: "chunk",
      id: `chunk:${chunk.chunk_id}`,
      label: chunk.key,
      value: chunk.key,
      detail: chunk.description ?? chunk.category ?? messages.promptChunk,
      rank: "workspace",
    }));
}

function tagItem(entry: PromptLexiconEntryDto): PromptCompletionItem {
  return {
    kind: "tag",
    id: `tag:${entry.tag}`,
    label: entry.tag,
    value: entry.tag,
    detail: entry.primary_translation || entry.category,
    rank: entry.match_rank,
  };
}

function completionForItem(
  item: PromptCompletionItem,
  context: PromptCompletionContext,
): Completion {
  const label =
    context.mode === "tag" && item.kind === "chunk" ? `$chunk(${item.label})` : item.label;
  return {
    label,
    detail: item.detail ?? undefined,
    type: item.kind === "tag" ? "text" : "function",
    apply: (view, completion) => {
      const edit = buildPromptCompletionEdit(
        view.state.doc.toString(),
        view.state.selection.main.head,
        item,
        context.manual,
      );
      view.dispatch({
        changes: { from: edit.replaceStart, to: edit.replaceEnd, insert: edit.insert },
        selection: { anchor: edit.selectionStart },
        annotations: pickedCompletion.of(completion),
      });
    },
  };
}

function shouldFetchChunks(context: PromptCompletionContext): boolean {
  return context.mode === "chunk" || context.manual;
}

function shouldFetchTags(context: PromptCompletionContext): boolean {
  return context.mode === "tag" && context.query.trim().length > 0;
}

function chunkMatches(chunk: PromptChunkDto, query: string): boolean {
  return [chunk.key, chunk.category ?? "", chunk.description ?? "", chunk.content].some((value) =>
    normalize(value).includes(query),
  );
}

function normalize(value: string): string {
  return value.toLowerCase().replaceAll("_", " ").trim();
}

async function recover<T>(promise: Promise<T[]>): Promise<T[]> {
  try {
    return await promise;
  } catch (error: unknown) {
    frontendLogger.warn("Prompt completion source unavailable", {
      error:
        error instanceof Error ? { name: error.name, message: error.message } : { value: error },
    });
    return [];
  }
}
