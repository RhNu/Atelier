import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = process.cwd();
const sourceDirectory = join(root, "src", "features", "prompt-editor");
const grammar = join(sourceDirectory, "nai-prompt.grammar");
const generator = join(root, "node_modules", "@lezer", "generator", "src", "lezer-generator.cjs");
const formatter = join(root, "node_modules", "oxfmt", "bin", "oxfmt");
const formatterConfig = join(root, ".oxfmtrc.json");
const temporaryDirectory = mkdtempSync(join(tmpdir(), "atelier-prompt-parser-"));
const generatedParser = join(temporaryDirectory, "nai-prompt-parser.ts");

try {
  execFileSync(
    process.execPath,
    [generator, "--typeScript", "--output", generatedParser, grammar],
    {
      stdio: "pipe",
    },
  );
  execFileSync(
    process.execPath,
    [
      formatter,
      "--config",
      formatterConfig,
      generatedParser,
      join(temporaryDirectory, "nai-prompt-parser.terms.ts"),
    ],
    { stdio: "pipe" },
  );
  for (const filename of ["nai-prompt-parser.ts", "nai-prompt-parser.terms.ts"]) {
    const expected = readFileSync(join(sourceDirectory, filename), "utf8");
    const actual = readFileSync(join(temporaryDirectory, filename), "utf8");
    if (actual !== expected) {
      throw new Error(
        `${filename} has drifted from nai-prompt.grammar; run pnpm prompt-parser:generate.`,
      );
    }
  }
} finally {
  rmSync(temporaryDirectory, { recursive: true, force: true });
}
