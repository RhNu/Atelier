type FlushDraft = () => Promise<void>;

const editors = new Set<FlushDraft>();
const pending = new Set<Promise<void>>();

export function registerGenerationDraftFlush(flush: FlushDraft): () => void {
  editors.add(flush);
  return () => {
    editors.delete(flush);
  };
}

export function trackGenerationDraftSave(save: Promise<void>): Promise<void> {
  pending.add(save);
  void save.then(
    () => pending.delete(save),
    () => undefined,
  );
  return save;
}

export async function flushGenerationDraft(): Promise<void> {
  const saves = [...pending, ...Array.from(editors, (flush) => flush())];
  const results = await Promise.allSettled(saves);
  for (const save of saves) pending.delete(save);
  const failure = results.find((result) => result.status === "rejected");
  if (failure?.status === "rejected") throw failure.reason;
}
