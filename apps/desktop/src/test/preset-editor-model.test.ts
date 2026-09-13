import {
  blankPresetEditorDraft,
  editorDraftToUpsertRequest,
  presetToEditorDraft,
} from "../features/resources/preset-editor-model";
import type { PromptPresetDto } from "../types";

describe("preset editor model", () => {
  it("serializes only the active prompt behavior buffers", () => {
    const draft = blankPresetEditorDraft("main");
    draft.prompt = {
      mode: "replace",
      before: "preserved before",
      after: "preserved after",
      replacement: "replacement",
    };
    draft.uc = {
      mode: "surround",
      before: "bad anatomy",
      after: "lowres",
      replacement: "preserved replacement",
    };

    const request = editorDraftToUpsertRequest(draft, "main");

    expect(request.prompt_behavior).toEqual({ mode: "replace", text: "replacement" });
    expect(request.uc_behavior).toEqual({
      mode: "surround",
      before: "bad anatomy",
      after: "lowres",
    });
  });

  it("maps explicit DTO behavior into editor-owned buffers", () => {
    const preset: PromptPresetDto = {
      preset_id: "preset-1",
      kind: "character",
      path: "hero",
      folder_id: null,
      identifier: "hero",
      display_name: "Hero",
      aliases: [],
      description: null,
      prompt_behavior: { mode: "replace", text: "1girl" },
      uc_behavior: { mode: "surround", before: "bad anatomy", after: "" },
      quality_override: null,
      uc_preset_override: null,
      preview: null,
      created_at_ms: 1,
      updated_at_ms: 1,
      models: ["nai-diffusion-4-5-full"],
    };

    const draft = presetToEditorDraft(preset);

    expect(draft.prompt).toEqual({
      mode: "replace",
      before: "",
      after: "",
      replacement: "1girl",
    });
    expect(draft.uc.mode).toBe("surround");
  });

  it("uses the required identifier when the optional display name is blank", () => {
    const draft = blankPresetEditorDraft("main");
    draft.identifier = "cinematic";

    const request = editorDraftToUpsertRequest(draft, "main");

    expect(request.path).toBe("cinematic");
    expect(request.display_name).toBe("cinematic");
  });
});
