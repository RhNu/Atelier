import {
  acceptCompletion,
  autocompletion,
  CompletionContext,
  completionStatus,
  currentCompletions,
  pickedCompletion,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { EditorState, Transaction } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { QueryClient } from "@tanstack/react-query";

import type {
  ListPromptChunksRequestDto,
  PromptChunkPageDto,
  PromptLexiconPageDto,
  PromptLexiconSearchQueryDto,
} from "@/types";

import { activatePromptArgumentsOnCompletion, createNaiPromptCompletion } from "./completion";

const mocks = vi.hoisted(() => ({
  listChunks: vi.fn<(request: ListPromptChunksRequestDto) => Promise<PromptChunkPageDto>>(),
  lexiconSearch: vi.fn<(request: PromptLexiconSearchQueryDto) => Promise<PromptLexiconPageDto>>(),
}));

vi.mock("@/platform/atelier", () => ({
  promptApi: mocks,
  queryKeys: {
    prompt: {
      chunks: (request: ListPromptChunksRequestDto) => ["prompt", "chunks", request],
      lexiconSearch: (request: PromptLexiconSearchQueryDto) => [
        "prompt",
        "lexicon-search",
        request,
      ],
    },
  },
}));

const messages = { reusableChunk: "Reusable chunk", promptChunk: "Prompt chunk" };

describe("CodeMirror prompt completion source", () => {
  beforeEach(() => {
    mocks.listChunks.mockReset();
    mocks.lexiconSearch.mockReset();
    mocks.listChunks.mockResolvedValue(chunkPage());
    mocks.lexiconSearch.mockImplementation(async (request) => lexiconPage(`${request.query}_tag`));
  });

  it("queries the complete current term instead of reusing first-character results", async () => {
    const source = createNaiPromptCompletion(queryClient(), messages);
    await complete(source, "c", false);
    await complete(source, "cine", false);

    expect(mocks.lexiconSearch.mock.calls.map(([request]) => request.query)).toEqual(["c", "cine"]);
  });

  it("keeps usable candidates when an independent source fails", async () => {
    mocks.listChunks.mockRejectedValue(new Error("workspace unavailable"));
    const source = createNaiPromptCompletion(queryClient(), messages);
    const tagResult = await complete(source, "cine", true);
    expect(tagResult?.options.map((option) => option.label)).toContain("cine_tag");

    mocks.listChunks.mockResolvedValue(chunkPage());
    mocks.lexiconSearch.mockRejectedValue(new Error("lexicon unavailable"));
    const chunkResult = await complete(
      createNaiPromptCompletion(queryClient(), messages),
      "li",
      true,
    );
    expect(chunkResult?.options.map((option) => option.label)).toContain("$chunk(lighting)");
  });

  it("applies the real token range, final caret, and pickedCompletion annotation", async () => {
    mocks.lexiconSearch.mockResolvedValue(lexiconPage("cinematic_lighting"));
    const source = createNaiPromptCompletion(queryClient(), messages);
    const state = EditorState.create({ doc: "cine,solo", selection: { anchor: 2 } });
    const result = await source(new CompletionContext(state, 2, false));
    const completion = result?.options[0];
    expect(completion).toBeDefined();

    let pickedLabel: string | undefined;
    const view = new EditorView({
      state: EditorState.create({
        doc: "cine,solo",
        selection: { anchor: 2 },
        extensions: EditorView.updateListener.of((update) => {
          for (const transaction of update.transactions) {
            pickedLabel = transaction.annotation(pickedCompletion)?.label ?? pickedLabel;
          }
        }),
      }),
      parent: document.body.appendChild(document.createElement("div")),
    });
    try {
      if (typeof completion?.apply !== "function")
        throw new Error("Expected a custom apply function");
      completion.apply(view, completion, result?.from ?? 0, result?.to ?? 2);
      expect(view.state.doc.toString()).toBe("cinematic_lighting, solo");
      expect(view.state.selection.main.head).toBe(20);
      expect(pickedLabel).toBe("cinematic_lighting");
    } finally {
      view.destroy();
      view.dom.remove();
    }
  });

  it("discards an older async response after the document query changes", async () => {
    const first = deferred<PromptLexiconPageDto>();
    mocks.lexiconSearch.mockImplementation((request) =>
      request.query === "c" ? first.promise : Promise.resolve(lexiconPage("cine_result")),
    );
    const source = createNaiPromptCompletion(queryClient(), messages);
    const parent = document.body.appendChild(document.createElement("div"));
    const view = new EditorView({
      parent,
      state: EditorState.create({
        extensions: autocompletion({
          override: [source],
          activateOnTyping: true,
          activateOnTypingDelay: 0,
        }),
      }),
    });
    try {
      insert(view, "c");
      await vi.waitFor(() =>
        expect(mocks.lexiconSearch).toHaveBeenCalledWith({ query: "c", limit: 20 }),
      );
      insert(view, "ine");
      await vi.waitFor(() =>
        expect(mocks.lexiconSearch).toHaveBeenCalledWith({ query: "cine", limit: 20 }),
      );
      first.resolve(lexiconPage("stale_result"));
      await vi.waitFor(() => expect(completionStatus(view.state)).toBe("active"));
      expect(currentCompletions(view.state).map((completion) => completion.label)).toEqual([
        "cine_result",
      ]);
    } finally {
      view.destroy();
      parent.remove();
    }
  });

  it("opens argument candidates immediately after accepting a function completion", async () => {
    const source = createNaiPromptCompletion(queryClient(), messages);
    const parent = document.body.appendChild(document.createElement("div"));
    const view = new EditorView({
      parent,
      state: EditorState.create({
        extensions: autocompletion({
          override: [source],
          activateOnTyping: true,
          activateOnTypingDelay: 0,
          activateOnCompletion: activatePromptArgumentsOnCompletion,
          interactionDelay: 0,
        }),
      }),
    });
    try {
      insert(view, "$ch");
      await vi.waitFor(() => expect(currentCompletions(view.state)).toHaveLength(1));

      expect(acceptCompletion(view)).toBe(true);
      expect(view.state.doc.toString()).toBe("$chunk(");
      await vi.waitFor(() =>
        expect(currentCompletions(view.state).map((completion) => completion.label)).toEqual([
          "lighting",
        ]),
      );
    } finally {
      view.destroy();
      parent.remove();
    }
  });

  it("opens argument candidates when the opening parenthesis is typed", async () => {
    const source = createNaiPromptCompletion(queryClient(), messages);
    const parent = document.body.appendChild(document.createElement("div"));
    const view = new EditorView({
      parent,
      state: EditorState.create({
        extensions: autocompletion({
          override: [source],
          activateOnTyping: true,
          activateOnTypingDelay: 0,
          activateOnCompletion: activatePromptArgumentsOnCompletion,
          interactionDelay: 0,
        }),
      }),
    });
    try {
      insert(view, "$chunk");
      await vi.waitFor(() => expect(currentCompletions(view.state)).toHaveLength(1));
      insert(view, "(");

      await vi.waitFor(() =>
        expect(currentCompletions(view.state).map((completion) => completion.label)).toEqual([
          "lighting",
        ]),
      );
    } finally {
      view.destroy();
      parent.remove();
    }
  });
});

type PromptCompletionSource = ReturnType<typeof createNaiPromptCompletion>;

async function complete(
  source: PromptCompletionSource,
  doc: string,
  explicit: boolean,
): Promise<CompletionResult | null> {
  const state = EditorState.create({ doc, selection: { anchor: doc.length } });
  return await source(new CompletionContext(state, doc.length, explicit));
}

function insert(view: EditorView, text: string) {
  const head = view.state.selection.main.head;
  view.dispatch({
    changes: { from: head, insert: text },
    selection: { anchor: head + text.length },
    annotations: Transaction.userEvent.of("input.type"),
  });
}

function queryClient(): QueryClient {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

function lexiconPage(tag: string): PromptLexiconPageDto {
  return {
    items: [
      {
        tag,
        weight: 1000,
        category: "general",
        subcategory: "",
        primary_translation: tag,
        matched_translation: tag,
        match_field: "tag",
        match_rank: "prefix",
      },
    ],
    total: 1,
    offset: 0,
    limit: 20,
  };
}

function chunkPage(): PromptChunkPageDto {
  return {
    items: [
      {
        chunk_id: "chunk-lighting",
        key: "lighting",
        content: "dramatic lighting",
        category: null,
        description: null,
        preview: null,
        created_at_ms: 1,
        updated_at_ms: 1,
      },
    ],
    total: 1,
    offset: 0,
    limit: 200,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}
