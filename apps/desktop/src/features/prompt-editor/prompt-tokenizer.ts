export type PromptTokenKind =
  | "whitespace"
  | "number"
  | "invalid_number"
  | "tag"
  | "identifier"
  | "string"
  | "unterminated_string"
  | "escaped"
  | "double_pipe"
  | "double_colon"
  | "symbol"
  | "text";

export type PromptToken = { kind: PromptTokenKind; from: number; to: number; text: string };

const NUMBER = /^-?\d+(?:\.\d+)?/u;
const INVALID_NUMBER = /^-?\d+(?:\.\d*){2,}/u;
const TAG = /^\d+[\p{ID_Continue}_-]+/u;
const IDENTIFIER = /^[\p{ID_Start}_][\p{ID_Continue}_-]*/u;
const SPECIAL = /[{}[\](),|@$=:"\\\s]/u;

export function tokenizePrompt(text: string): PromptToken[] {
  const tokens: PromptToken[] = [];
  let index = 0;
  while (index < text.length) {
    const from = index;
    const char = text[index];
    if (char === undefined) break;
    if (/\s/u.test(char)) {
      while (index < text.length && /\s/u.test(text[index] ?? "")) index += 1;
      push(tokens, "whitespace", text, from, index);
      continue;
    }
    if (text.startsWith("||", index) || text.startsWith("::", index)) {
      index += 2;
      push(
        tokens,
        text.slice(from, index) === "||" ? "double_pipe" : "double_colon",
        text,
        from,
        index,
      );
      continue;
    }
    if (char === "\\" && index + 1 < text.length) {
      index += 2;
      push(tokens, "escaped", text, from, index);
      continue;
    }
    if (char === '"') {
      index += 1;
      let closed = false;
      while (index < text.length) {
        if (text[index] === "\\") index += 2;
        else if (text[index++] === '"') {
          closed = true;
          break;
        } else index += 1;
      }
      push(
        tokens,
        closed ? "string" : "unterminated_string",
        text,
        from,
        Math.min(index, text.length),
      );
      continue;
    }
    const slice = text.slice(index);
    const invalid = INVALID_NUMBER.exec(slice)?.[0];
    const numberCandidate = NUMBER.exec(slice)?.[0];
    const number =
      numberCandidate && /[\p{ID_Continue}_-]/u.test(slice[numberCandidate.length] ?? "")
        ? undefined
        : numberCandidate;
    const tag = TAG.exec(slice)?.[0];
    const identifier = IDENTIFIER.exec(slice)?.[0];
    const matched = invalid ?? number ?? tag ?? identifier;
    if (matched) {
      index += matched.length;
      push(
        tokens,
        invalid ? "invalid_number" : number ? "number" : tag ? "tag" : "identifier",
        text,
        from,
        index,
      );
      continue;
    }
    if ("{}[](),|@$=:".includes(char)) {
      index += 1;
      push(tokens, "symbol", text, from, index);
      continue;
    }
    index += 1;
    while (index < text.length && !SPECIAL.test(text[index] ?? "")) index += 1;
    push(tokens, "text", text, from, index);
  }
  return tokens;
}

function push(
  tokens: PromptToken[],
  kind: PromptTokenKind,
  text: string,
  from: number,
  to: number,
) {
  tokens.push({ kind, from, to, text: text.slice(from, to) });
}
