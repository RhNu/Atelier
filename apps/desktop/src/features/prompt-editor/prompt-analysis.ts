import type { Diagnostic } from "@codemirror/lint";

export type NaiPromptProfile = "novelai_v3" | "novelai_v4" | "novelai_v45";

export function promptProfileForModel(model: string): NaiPromptProfile {
  if (model.includes("4-5")) return "novelai_v45";
  if (model.includes("diffusion-4")) return "novelai_v4";
  return "novelai_v3";
}

type TokenKind =
  | "whitespace"
  | "number"
  | "invalid_number"
  | "identifier"
  | "string"
  | "unterminated_string"
  | "escaped"
  | "double_pipe"
  | "double_colon"
  | "symbol"
  | "text";

export type PromptToken = { kind: TokenKind; from: number; to: number; text: string };
export type PromptAnalysis = { tokens: PromptToken[]; diagnostics: Diagnostic[] };

const NUMBER = /^-?\d+(?:\.\d+)?/u;
const INVALID_NUMBER = /^-?\d+(?:\.\d*){2,}/u;
const IDENTIFIER = /^[\p{ID_Start}_][\p{ID_Continue}_-]*/u;
const SPECIAL = /[{}[\](),|@$=:"\\\s]/u;

export function analyzePrompt(text: string, profile: NaiPromptProfile): PromptAnalysis {
  const tokens = tokenizePrompt(text);
  const diagnostics: Diagnostic[] = [];
  const stack: Array<{ kind: "brace" | "bracket"; token: PromptToken }> = [];
  let extensionDepth = 0;

  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (!token) continue;
    const previous = tokens[index - 1];
    if (
      token.text === "$" &&
      tokens[index + 1]?.kind === "identifier" &&
      tokens[index + 2]?.text === "("
    ) {
      extensionDepth += 1;
    } else if (extensionDepth > 0 && token.text === ")") {
      extensionDepth -= 1;
      continue;
    }
    if (extensionDepth > 0) continue;

    if (token.text === "{") stack.push({ kind: "brace", token });
    if (token.text === "[") stack.push({ kind: "bracket", token });
    if (token.text === "}" && !popMatching(stack, "brace")) {
      diagnostics.push(
        error(
          token,
          "unmatched_strengthening_close",
          "Strengthening close delimiter has no matching opener.",
        ),
      );
    }
    if (token.text === "]" && !popMatching(stack, "bracket")) {
      diagnostics.push(
        error(
          token,
          "unmatched_weakening_close",
          "Weakening close delimiter has no matching opener.",
        ),
      );
    }
    if (token.kind === "invalid_number" && tokens[index + 1]?.kind === "double_colon") {
      diagnostics.push(
        error(token, "invalid_numeric_weight", "Numeric emphasis weight is invalid."),
      );
    }
    if (token.kind === "number" && tokens[index + 1]?.kind === "double_colon") {
      const value = Number(token.text);
      if (profile === "novelai_v3") {
        diagnostics.push(
          error(
            token,
            "unsupported_capability",
            "Numeric emphasis is not supported by this model.",
          ),
        );
      } else if (value < 0 && profile !== "novelai_v45") {
        diagnostics.push(
          error(
            token,
            "unsupported_capability",
            "Negative numeric emphasis requires NAI Diffusion 4.5.",
          ),
        );
      }
      if (!findClosingDoubleColon(tokens, index + 2)) {
        diagnostics.push(
          warning(
            token,
            "unclosed_numeric_emphasis",
            "Numeric emphasis remains active until the end of the prompt.",
          ),
        );
      }
    }
    if (token.kind === "unterminated_string") {
      diagnostics.push(error(token, "unterminated_string", "String literal is not closed."));
    }
    if (token.kind === "double_pipe" && previous?.kind !== "double_pipe") {
      const close = findToken(tokens, index + 1, "double_pipe");
      if (close < 0) {
        diagnostics.push(warning(token, "unclosed_randomizer", "Prompt randomizer is not closed."));
      } else if (hasEmptyRandomizerOption(tokens, index + 1, close)) {
        diagnostics.push(
          error(token, "empty_randomizer_option", "Prompt randomizer contains an empty option."),
        );
      }
    }
    if (
      token.text === "|" &&
      profile !== "novelai_v3" &&
      profile !== "novelai_v4" &&
      profile !== "novelai_v45"
    ) {
      diagnostics.push(
        error(token, "ambiguous_pipe", "Pipe cannot be interpreted by this syntax profile."),
      );
    }
  }

  for (const open of stack) {
    diagnostics.push(
      warning(
        open.token,
        open.kind === "brace" ? "unclosed_strengthening" : "unclosed_weakening",
        open.kind === "brace"
          ? "Strengthening block is not closed."
          : "Weakening block is not closed.",
      ),
    );
  }
  if (extensionDepth > 0) {
    const at = [...tokens].reverse().find((token) => token.text === "$");
    if (at)
      diagnostics.push(
        warning(at, "unclosed_function_call", "Extension function call is not closed."),
      );
  }
  return { tokens, diagnostics };
}

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
    const identifier = IDENTIFIER.exec(slice)?.[0];
    const matched = invalid ?? number ?? identifier;
    if (matched) {
      index += matched.length;
      push(
        tokens,
        invalid ? "invalid_number" : number ? "number" : "identifier",
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

function push(tokens: PromptToken[], kind: TokenKind, text: string, from: number, to: number) {
  tokens.push({ kind, from, to, text: text.slice(from, to) });
}
function popMatching(
  stack: Array<{ kind: "brace" | "bracket"; token: PromptToken }>,
  kind: "brace" | "bracket",
) {
  const found = stack.findLastIndex((entry) => entry.kind === kind);
  if (found < 0) return false;
  stack.splice(found, 1);
  return true;
}
function findToken(tokens: PromptToken[], start: number, kind: TokenKind) {
  return tokens.findIndex((token, index) => index >= start && token.kind === kind);
}
function findClosingDoubleColon(tokens: PromptToken[], start: number) {
  return findToken(tokens, start, "double_colon") >= 0;
}
function hasEmptyRandomizerOption(tokens: PromptToken[], start: number, end: number) {
  let hasContent = false;
  for (let index = start; index < end; index += 1) {
    const token = tokens[index];
    if (!token) continue;
    if (token.text === "|") {
      if (!hasContent) return true;
      hasContent = false;
    } else if (token.kind !== "whitespace") hasContent = true;
  }
  return !hasContent;
}
function error(token: PromptToken, code: string, message: string): Diagnostic {
  return {
    from: token.from,
    to: Math.max(token.to, token.from + 1),
    severity: "error",
    message,
    source: code,
  };
}
function warning(token: PromptToken, code: string, message: string): Diagnostic {
  return {
    from: token.from,
    to: Math.max(token.to, token.from + 1),
    severity: "warning",
    message,
    source: code,
  };
}
