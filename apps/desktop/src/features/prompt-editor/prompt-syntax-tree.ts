import { naiPromptLanguage } from "./language";

export type PromptSyntaxFrame = { name: string; from: number; to: number };
export type PromptSyntaxRecord = PromptSyntaxFrame & { ancestors: PromptSyntaxFrame[] };
export type PromptSyntax = { nodes: PromptSyntaxRecord[]; leaves: PromptSyntaxRecord[] };

type PromptTree = ReturnType<typeof naiPromptLanguage.parser.parse>;

export function inspectPromptTree(tree: PromptTree): PromptSyntax {
  const nodes: PromptSyntaxRecord[] = [];
  const leaves: PromptSyntaxRecord[] = [];
  const cursor = tree.cursor();

  const visit = (ancestors: PromptSyntaxFrame[]) => {
    const frame = { name: cursor.name, from: cursor.from, to: cursor.to };
    const record = { ...frame, ancestors };
    nodes.push(record);
    if (cursor.firstChild()) {
      const childAncestors = [...ancestors, frame];
      do visit(childAncestors);
      while (cursor.nextSibling());
      cursor.parent();
    } else {
      leaves.push(record);
    }
  };

  visit([]);
  return { nodes, leaves };
}

export function promptDescendants(
  syntax: PromptSyntax,
  ancestor: PromptSyntaxFrame,
  name: string,
): PromptSyntaxRecord[] {
  return syntax.leaves.filter(
    (leaf) =>
      leaf.name === name &&
      leaf.ancestors.some((item) => syntaxRecordKey(item) === syntaxRecordKey(ancestor)),
  );
}

export function firstPromptDescendant(
  syntax: PromptSyntax,
  ancestor: PromptSyntaxFrame,
  name: string,
): PromptSyntaxRecord | undefined {
  return promptDescendants(syntax, ancestor, name)[0];
}

export function hasPromptAncestor(record: PromptSyntaxRecord, name: string): boolean {
  return record.ancestors.some((item) => item.name === name);
}

export function syntaxRecordKey(record: PromptSyntaxFrame): string {
  return `${record.name}:${record.from}:${record.to}`;
}
