import type { Completion, CompletionContext, CompletionResult } from "@codemirror/autocomplete";

import {
  buildPromptCompletionEdit,
  getPromptCompletionContext,
  type PromptCompletionItem,
} from "@/features/generation/components/prompt-completion/prompt-completion-utils";
import { promptApi } from "@/platform/atelier";

const FUNCTION_OPTIONS = [
  {
    label: "chunk",
    detail: "Reusable prompt chunk",
    type: "function" as const,
  },
];

export async function naiPromptCompletion(
  context: CompletionContext,
): Promise<CompletionResult | null> {
  const source = context.state.doc.toString();
  const promptContext = getPromptCompletionContext(source, context.pos, context.explicit);

  if (promptContext.mode === "function") {
    const query = promptContext.query.toLowerCase();
    const options = FUNCTION_OPTIONS.filter((item) => item.label.includes(query)).map((item) =>
      withPromptApply(item, {
        kind: "function",
        id: `function:${item.label}`,
        label: item.label,
        value: item.label,
        detail: item.detail,
        rank: "function",
      }),
    );
    return options.length > 0
      ? {
          from: promptContext.replaceStart,
          options,
          validFor: /^[A-Za-z0-9_-]*$/u,
        }
      : null;
  }

  if (promptContext.mode === "chunk") {
    const page = await promptApi.listChunks({ offset: 0, limit: 200 });
    const query = normalize(promptContext.query);
    const options = page.items
      .filter(
        (item) =>
          !query ||
          normalize(`${item.key} ${item.category ?? ""} ${item.description ?? ""}`).includes(query),
      )
      .slice(0, 20)
      .map((item) =>
        withPromptApply(
          {
            label: item.key,
            detail: item.description ?? item.category ?? "Prompt chunk",
            type: "function" as const,
          },
          {
            kind: "chunk",
            id: `chunk:${item.chunk_id}`,
            label: item.key,
            value: item.key,
            detail: item.description ?? item.category ?? "Prompt chunk",
            rank: "workspace",
          },
        ),
      );
    return options.length > 0
      ? {
          from: promptContext.replaceStart,
          options,
          validFor: /^[^,()[\]{}|\s]*$/u,
        }
      : null;
  }

  const query = promptContext.query.trim();
  if (!context.explicit && query.length === 0) return null;
  const [chunks, lexicon] = await Promise.all([
    context.explicit
      ? promptApi.listChunks({ offset: 0, limit: 20 })
      : Promise.resolve({ items: [] }),
    query ? promptApi.lexiconSearch({ query, limit: 20 }) : Promise.resolve({ items: [] }),
  ]);
  const options: Completion[] = [
    ...(context.explicit
      ? FUNCTION_OPTIONS.map((item) =>
          withPromptApply(item, {
            kind: "function",
            id: `function:${item.label}`,
            label: item.label,
            value: item.label,
            detail: item.detail,
            rank: "function",
          }),
        )
      : []),
    ...chunks.items.map((item) =>
      withPromptApply(
        {
          label: `$chunk(${item.key})`,
          detail: item.description ?? item.category ?? "Prompt chunk",
          type: "function" as const,
        },
        {
          kind: "chunk",
          id: `chunk:${item.chunk_id}`,
          label: item.key,
          value: item.key,
          detail: item.description ?? item.category ?? "Prompt chunk",
          rank: "workspace",
        },
      ),
    ),
    ...lexicon.items.map((item) =>
      withPromptApply(
        {
          label: item.tag,
          detail: item.primary_translation || item.category,
          type: "text" as const,
        },
        {
          kind: "tag",
          id: `tag:${item.tag}`,
          label: item.tag,
          value: item.tag,
          detail: item.primary_translation || item.category,
          rank: item.match_rank,
        },
      ),
    ),
  ];
  return options.length > 0
    ? {
        from: promptContext.replaceStart,
        options,
        validFor: /^[^,\n\r|{}[\]]*$/u,
      }
    : null;
}

function withPromptApply(
  option: { label: string; detail: string | null; type: "function" | "text" },
  item: PromptCompletionItem,
): Completion {
  return {
    ...option,
    detail: option.detail ?? undefined,
    apply: (view, _completion, _from, to) => {
      const source = view.state.doc.toString();
      const edit = buildPromptCompletionEdit(source, to, item);
      view.dispatch({
        changes: { from: edit.replaceStart, to: edit.replaceEnd, insert: edit.insert },
        selection: { anchor: edit.selectionStart },
      });
    },
  };
}

function normalize(value: string): string {
  return value.toLowerCase().replaceAll("_", " ").trim();
}
