import type { Diagnostic } from "@codemirror/lint";

import type { NaiPromptProfile, PromptEditorMessages } from "./prompt-analysis";
import {
  firstPromptDescendant,
  hasPromptAncestor,
  promptDescendants,
  type PromptSyntax,
  type PromptSyntaxFrame,
} from "./prompt-syntax-tree";

export function buildPromptDiagnostics(
  text: string,
  syntax: PromptSyntax,
  profile: NaiPromptProfile,
  messages: PromptEditorMessages,
): Diagnostic[] {
  const diagnostics: Diagnostic[] = [];
  diagnoseBlocks(syntax, diagnostics, messages);
  diagnoseStrings(syntax, diagnostics, messages);
  diagnoseNumericWeights(text, syntax, profile, diagnostics, messages);
  diagnoseRandomizers(text, syntax, diagnostics, messages);
  diagnoseFunctions(text, syntax, diagnostics, messages);
  return diagnostics;
}

function diagnoseBlocks(
  syntax: PromptSyntax,
  diagnostics: Diagnostic[],
  messages: PromptEditorMessages,
) {
  for (const node of syntax.nodes) {
    const strengtheningOpen = firstPromptDescendant(syntax, node, "LBrace");
    const weakeningOpen = firstPromptDescendant(syntax, node, "LBracket");
    if (node.name === "Strengthening" && !firstPromptDescendant(syntax, node, "RBrace")) {
      diagnostics.push(
        warning(
          strengtheningOpen ?? node,
          "unclosed_strengthening",
          messages.unclosedStrengthening,
        ),
      );
    }
    if (node.name === "Weakening" && !firstPromptDescendant(syntax, node, "RBracket")) {
      diagnostics.push(
        warning(weakeningOpen ?? node, "unclosed_weakening", messages.unclosedWeakening),
      );
    }
  }
  for (const leaf of syntax.leaves) {
    if (leaf.name === "RBrace" && !hasPromptAncestor(leaf, "Strengthening")) {
      diagnostics.push(
        error(leaf, "unmatched_strengthening_close", messages.unmatchedStrengtheningClose),
      );
    }
    if (leaf.name === "RBracket" && !hasPromptAncestor(leaf, "Weakening")) {
      diagnostics.push(error(leaf, "unmatched_weakening_close", messages.unmatchedWeakeningClose));
    }
  }
}

function diagnoseStrings(
  syntax: PromptSyntax,
  diagnostics: Diagnostic[],
  messages: PromptEditorMessages,
) {
  for (const leaf of syntax.leaves) {
    if (leaf.name === "UnterminatedString") {
      diagnostics.push(error(leaf, "unterminated_string", messages.unterminatedString));
    }
  }
}

function diagnoseNumericWeights(
  text: string,
  syntax: PromptSyntax,
  profile: NaiPromptProfile,
  diagnostics: Diagnostic[],
  messages: PromptEditorMessages,
) {
  for (const node of syntax.nodes.filter((item) => item.name === "NumericEmphasis")) {
    const number =
      firstPromptDescendant(syntax, node, "Number") ??
      firstPromptDescendant(syntax, node, "InvalidNumber");
    if (!number) continue;
    if (number.name === "InvalidNumber") {
      diagnostics.push(error(number, "invalid_numeric_weight", messages.invalidNumericWeight));
    }
    if (profile === "novelai_v3") {
      diagnostics.push(error(number, "unsupported_capability", messages.unsupportedNumericWeight));
    } else if (text.slice(number.from, number.to).startsWith("-") && profile !== "novelai_v45") {
      diagnostics.push(
        error(number, "unsupported_capability", messages.unsupportedNegativeNumericWeight),
      );
    }
    if (promptDescendants(syntax, node, "DoubleColon").length < 2) {
      diagnostics.push(
        warning(number, "unclosed_numeric_emphasis", messages.unclosedNumericWeight),
      );
    }
  }
}

function diagnoseRandomizers(
  text: string,
  syntax: PromptSyntax,
  diagnostics: Diagnostic[],
  messages: PromptEditorMessages,
) {
  for (const node of syntax.nodes.filter((item) => item.name === "Randomizer")) {
    const delimiters = promptDescendants(syntax, node, "DoublePipe");
    const [open, close] = delimiters;
    if (!open || !close) {
      diagnostics.push(warning(open ?? node, "unclosed_randomizer", messages.unclosedRandomizer));
      continue;
    }
    const pipes = promptDescendants(syntax, node, "Pipe").filter(
      (pipe) => !hasPromptAncestor(pipe, "ExtensionCall"),
    );
    let optionStart = open.to;
    const hasEmptyOption = [...pipes, close].some((delimiter) => {
      const empty = text.slice(optionStart, delimiter.from).trim().length === 0;
      optionStart = delimiter.to;
      return empty;
    });
    if (hasEmptyOption) {
      diagnostics.push(error(open, "empty_randomizer_option", messages.emptyRandomizerOption));
    }
  }
}

function diagnoseFunctions(
  text: string,
  syntax: PromptSyntax,
  diagnostics: Diagnostic[],
  messages: PromptEditorMessages,
) {
  for (const node of syntax.nodes.filter((item) => item.name === "ExtensionCall")) {
    const name = firstPromptDescendant(syntax, node, "Identifier");
    const open = firstPromptDescendant(syntax, node, "LParen");
    const close = firstPromptDescendant(syntax, node, "RParen");
    if (!close) {
      diagnostics.push(warning(node, "unclosed_function_call", messages.unclosedFunctionCall));
    }
    if (!name || !open) continue;
    if (text.slice(name.from, name.to) !== "chunk") {
      diagnostics.push(error(name, "unknown_function", messages.unknownFunction));
      continue;
    }
    const argumentEnd = close?.from ?? node.to;
    const commas = promptDescendants(syntax, node, "Comma").filter(
      (comma) => comma.from < argumentEnd,
    );
    const boundaries = [open.to, ...commas.map((comma) => comma.from), argumentEnd];
    const arguments_ = boundaries
      .slice(1)
      .map((to, index) => text.slice(boundaries[index], to).trim())
      .filter(Boolean);
    if (arguments_.length !== 1) {
      diagnostics.push(error(node, "invalid_function_arity", messages.invalidFunctionArity));
    }
    for (const argument of arguments_) {
      const equals = argument.indexOf("=");
      if (equals >= 0 && argument.slice(0, equals).trim() !== "name") {
        diagnostics.push(
          error(node, "invalid_function_argument", messages.invalidFunctionArgument),
        );
      }
    }
  }
}

function error(range: PromptSyntaxFrame, code: string, message: string): Diagnostic {
  return diagnostic(range, code, message, "error");
}

function warning(range: PromptSyntaxFrame, code: string, message: string): Diagnostic {
  return diagnostic(range, code, message, "warning");
}

function diagnostic(
  range: PromptSyntaxFrame,
  code: string,
  message: string,
  severity: "error" | "warning",
): Diagnostic {
  return {
    from: range.from,
    to: Math.max(range.to, range.from + 1),
    severity,
    message,
    source: code,
  };
}
