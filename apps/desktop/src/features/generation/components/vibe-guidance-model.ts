import type { VibeDocumentEntryDto } from "../../../types";

export function findVibeEncodingForModel(entry: VibeDocumentEntryDto, model: string) {
  const index = entry.available_encoding_configs.findIndex((config) => config.model === model);
  if (index < 0) {
    return null;
  }
  const encoding = entry.encodings[index];
  const config = entry.available_encoding_configs[index];
  return encoding && config ? { encoding, config } : null;
}
