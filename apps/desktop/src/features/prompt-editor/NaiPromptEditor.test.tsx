import { diagnosticCount } from "@codemirror/lint";
import { runScopeHandlers } from "@codemirror/view";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";

import {
  clearPromptEditor,
  promptEditorText,
  promptEditorView,
  typeInPromptEditor,
} from "@/test/prompt-editor-test-utils";
import type { LexiconCompleteRequestDto, LexiconSearchItemDto, PromptChunkPageDto } from "@/types";

import { NaiPromptEditor, type NaiPromptHighlightMode } from "./NaiPromptEditor";
import type { NaiPromptProfile } from "./prompt-analysis";

const mocks = vi.hoisted(() => ({
  listChunks: vi.fn<() => Promise<PromptChunkPageDto>>(async () => ({
    items: [
      {
        chunk_id: "chunk-lighting",
        key: "lighting",
        content: "dramatic light",
        category: null,
        description: null,
        preview: null,
        created_at_ms: 1,
        updated_at_ms: 1,
        models: ["nai-diffusion-4-5-full"],
      },
    ],
    total: 1,
    offset: 0,
    limit: 200,
  })),
  lexiconComplete: vi.fn<(request: LexiconCompleteRequestDto) => Promise<LexiconSearchItemDto[]>>(
    async (request): Promise<LexiconSearchItemDto[]> => [
      {
        entity_id: 1,
        canonical_name: "cinematic_lighting",
        primary_translation: "cinematic lighting",
        kind: "tag",
        category: "general",
        post_count: 1000,
        rating: "safe",
        matched_text: request.query,
        match_reason: "canonical_prefix",
        score: 97,
      },
    ],
  ),
}));

vi.mock("@/platform/atelier", () => ({
  promptApi: { listChunks: mocks.listChunks },
  lexiconApi: { complete: mocks.lexiconComplete },
  queryKeys: {
    prompt: {
      chunks: (request: unknown) => ["prompt", "chunks", request],
    },
    lexicon: {
      completion: (query: string, limit: number) => ["lexicon", "completion", query, limit],
    },
  },
}));

describe("NaiPromptEditor", () => {
  it("reacts to controlled value, profile, read-only, placeholder, ARIA, and highlight changes", async () => {
    const onChange = vi.fn<(value: string) => void>();
    const view = render(
      editor({
        value: "1.2::portrait::",
        profile: "novelai_v3",
        placeholder: "First placeholder",
        ariaLabel: "First prompt",
        id: "first-prompt",
        onChange,
      }),
    );
    const content = screen.getByLabelText("First prompt");
    const editorView = promptEditorView(content);
    expect(content).toHaveAttribute("id", "first-prompt");
    expect(editorView.dom).toHaveClass("nai-highlight-foreground");
    await waitFor(() => expect(diagnosticCount(editorView.state)).toBe(1));

    view.rerender(
      editor({
        value: "",
        profile: "novelai_v45",
        placeholder: "Updated placeholder",
        ariaLabel: "Updated prompt",
        id: "updated-prompt",
        readOnly: true,
        highlightMode: "background",
        onChange,
      }),
    );
    const updated = screen.getByLabelText("Updated prompt");
    expect(promptEditorText(updated)).toBe("");
    expect(updated).toHaveAttribute("id", "updated-prompt");
    expect(updated).toHaveAttribute("contenteditable", "false");
    expect(editorView.dom).toHaveClass("nai-highlight-background");
    expect(editorView.dom).not.toHaveClass("nai-highlight-foreground");
    expect(screen.getByText("Updated placeholder")).toBeInTheDocument();
    await waitFor(() => expect(diagnosticCount(editorView.state)).toBe(0));
    expect(onChange).not.toHaveBeenCalled();
  });

  it("defers an external controlled value until IME composition ends", async () => {
    const view = render(editor({ value: "before", ariaLabel: "IME prompt" }));
    const content = screen.getByLabelText("IME prompt");
    fireEvent.compositionStart(content, { data: "中" });

    view.rerender(editor({ value: "external", ariaLabel: "IME prompt" }));
    expect(promptEditorText(content)).toBe("before");
    fireEvent.compositionEnd(content, { data: "中" });
    await waitFor(() => expect(promptEditorText(content)).toBe("external"));
  });

  it("highlights function markers together and weakens the complete comment call", async () => {
    render(
      editor({
        value: '$chunk(hero), $comment("draft")',
        ariaLabel: "Highlighted prompt",
      }),
    );

    const content = screen.getByLabelText("Highlighted prompt");
    await waitFor(() => {
      expect(highlightedText(content, ".nai-function:not(.nai-function-comment)")).toBe("$chunk");
      expect(highlightedText(content, ".nai-function-comment")).toBe('$comment("draft")');
    });
  });

  it("wires Tab, Enter, Escape, and Ctrl-Space through the CodeMirror completion state", async () => {
    render(
      <QueryClientProvider client={queryClient()}>
        <StatefulEditor />
      </QueryClientProvider>,
    );
    const content = screen.getByLabelText("Completion prompt");
    const editorView = promptEditorView(content);
    expect(runKey(editorView, "Tab")).toBe(false);

    expect(runKey(editorView, " ", { ctrlKey: true })).toBe(true);
    expect(await screen.findByRole("option", { name: /lighting/u })).toBeInTheDocument();
    expect(runKey(editorView, "Escape")).toBe(true);
    await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());

    typeInPromptEditor(content, "cine");
    await screen.findByRole("option", { name: /cinematic_lighting/u });
    expect(runKey(editorView, "Enter")).toBe(true);
    expect(promptEditorText(content)).toBe("cinematic_lighting, ");

    clearPromptEditor(content);
    typeInPromptEditor(content, "cine");
    await screen.findByRole("option", { name: /cinematic_lighting/u });
    expect(runKey(editorView, "Tab")).toBe(true);
    expect(promptEditorText(content)).toBe("cinematic_lighting, ");
  });
});

type EditorOptions = {
  value: string;
  ariaLabel: string;
  id?: string;
  profile?: NaiPromptProfile;
  placeholder?: string;
  readOnly?: boolean;
  highlightMode?: NaiPromptHighlightMode;
  onChange?: (value: string) => void;
};

function editor(options: EditorOptions) {
  return (
    <QueryClientProvider client={queryClient()}>
      <NaiPromptEditor
        id={options.id}
        aria-label={options.ariaLabel}
        value={options.value}
        onChange={options.onChange ?? ignoreChange}
        profile={options.profile}
        placeholder={options.placeholder}
        readOnly={options.readOnly}
        highlightMode={options.highlightMode}
      />
    </QueryClientProvider>
  );
}

function StatefulEditor() {
  const [value, setValue] = useState("");
  return <NaiPromptEditor aria-label="Completion prompt" value={value} onChange={setValue} />;
}

function queryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

function ignoreChange() {}

function highlightedText(content: HTMLElement, selector: string): string {
  return [...content.querySelectorAll(selector)].map((element) => element.textContent).join("");
}

function runKey(
  view: ReturnType<typeof promptEditorView>,
  key: string,
  init: KeyboardEventInit = {},
): boolean {
  let handled = false;
  act(() => {
    handled = runScopeHandlers(view, new KeyboardEvent("keydown", { key, ...init }), "editor");
  });
  return handled;
}
