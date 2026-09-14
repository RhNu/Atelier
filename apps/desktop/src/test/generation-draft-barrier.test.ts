import {
  flushGenerationDraft,
  registerGenerationDraftFlush,
  trackGenerationDraftSave,
} from "../features/generation/state/draft-save-barrier";

describe("generation draft save barrier", () => {
  it("waits for both mounted editors and saves from a departed route", async () => {
    const editor = deferred();
    const departed = deferred();
    const unregister = registerGenerationDraftFlush(() => editor.promise);
    void trackGenerationDraftSave(departed.promise);
    let finished = false;
    const barrier = flushGenerationDraft().then(() => {
      finished = true;
    });
    editor.resolve();
    await Promise.resolve();
    expect(finished).toBe(false);
    departed.resolve();
    await barrier;
    expect(finished).toBe(true);
    unregister();
  });

  it("reports failure only after every outstanding save has settled", async () => {
    const save = deferred();
    const unregisterFailure = registerGenerationDraftFlush(() =>
      Promise.reject(new Error("conflict")),
    );
    const unregisterSave = registerGenerationDraftFlush(() => save.promise);
    let failure: unknown;
    const barrier = flushGenerationDraft().catch((error: unknown) => {
      failure = error;
    });
    await Promise.resolve();
    expect(failure).toBeUndefined();
    save.resolve();
    await barrier;
    expect(failure).toEqual(new Error("conflict"));
    unregisterFailure();
    unregisterSave();
  });
});

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
