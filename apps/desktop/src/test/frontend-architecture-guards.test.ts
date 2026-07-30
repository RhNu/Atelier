import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";
import { describe, expect, it } from "vitest";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const srcRoot = path.join(projectRoot, "src");

const REQUIRED_FRONTEND_AREAS = [
  "src/platform/atelier",
  "src/components/ui",
  "src/features/generation",
  "src/features/director",
  "src/features/resources",
  "src/features/lexicon",
  "src/features/inspiration",
  "src/features/gallery",
  "src/features/settings",
  "src/routes",
] as const;

const REQUIRED_FRONTEND_DOCS = ["docs/agents/frontend-architecture.md"] as const;

const QUERY_HOOK_ALLOWED_PATH_PATTERNS = [
  /^src\/features\/[^/]+\/data\//u,
  /^src\/features\/[^/]+\/runtime\//u,
  /^src\/features\/workspace\//u,
  /^src\/platform\/atelier\//u,
] as const;
const TOP_LEVEL_PAGE_PATHS = [
  "src/features/director/DirectorPage.tsx",
  "src/features/gallery/GalleryPage.tsx",
  "src/features/lexicon/LexiconPage.tsx",
  "src/features/resources/ResourcesPage.tsx",
  "src/features/settings/SettingsPage.tsx",
] as const;
const USER_VISIBLE_JSX_ATTRIBUTES = new Set(["aria-label", "label", "placeholder"]);
const ALLOWED_LITERAL_LABELS = new Set([
  "Anlas",
  "NSFW",
  "Atelier",
  "A",
  "R",
  "S",
  "ID",
  "source",
  "mask",
  "resource:",
  "variant:",
  "vibe:",
]);

function walkFiles(dirPath: string): string[] {
  return readdirSync(dirPath)
    .flatMap((entry) => {
      const fullPath = path.join(dirPath, entry);
      const stat = statSync(fullPath);
      return stat.isDirectory() ? walkFiles(fullPath) : fullPath;
    })
    .sort((left, right) => left.localeCompare(right, "en"));
}

function toProjectPath(filePath: string): string {
  return path.relative(projectRoot, filePath).split(path.sep).join("/");
}

function readProjectFile(filePath: string): string {
  return readFileSync(filePath, "utf8");
}

function hasStringPermissions(value: unknown): value is { permissions: string[] } {
  return (
    typeof value === "object" &&
    value !== null &&
    "permissions" in value &&
    Array.isArray(value.permissions) &&
    value.permissions.every((permission) => typeof permission === "string")
  );
}

function hasCspDirectives(value: unknown): value is Record<string, string | string[]> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasProperty<Key extends PropertyKey>(
  value: unknown,
  key: Key,
): value is Record<Key, unknown> {
  return typeof value === "object" && value !== null && key in value;
}

function collectSourceFiles(): string[] {
  return walkFiles(srcRoot).filter((filePath) => {
    const projectPath = toProjectPath(filePath);
    return (
      /\.(css|ts|tsx)$/u.test(filePath) &&
      !projectPath.includes("/test/") &&
      !projectPath.includes(".test.") &&
      !projectPath.endsWith(".d.ts") &&
      !projectPath.includes("/types/generated/")
    );
  });
}

describe("frontend architecture guards", () => {
  it("keeps frontend architecture guidance documented and discoverable", () => {
    const missingDocs = REQUIRED_FRONTEND_DOCS.filter(
      (relativePath) => !existsSync(path.join(projectRoot, "..", "..", relativePath)),
    );
    const agentsReadme = readProjectFile(
      path.join(projectRoot, "..", "..", "docs/agents/README.md"),
    );

    expect(missingDocs).toEqual([]);
    expect(agentsReadme).toContain("frontend-architecture.md");
  });

  it("keeps the planned frontend foundation areas present", () => {
    const missingPaths = REQUIRED_FRONTEND_AREAS.filter(
      (relativePath) => !existsSync(path.join(projectRoot, relativePath)),
    );

    expect(missingPaths).toEqual([]);
  });

  it("keeps views and feature pages away from direct Tauri calls", () => {
    const offenders = collectSourceFiles()
      .filter((filePath) => /^src\/(?:features|routes)\//u.test(toProjectPath(filePath)))
      .filter((filePath) => readProjectFile(filePath).includes("@tauri-apps/api"))
      .map(toProjectPath);

    expect(offenders).toEqual([]);
  });

  it("restricts query hooks to data/runtime/platform modules", () => {
    const offenders = collectSourceFiles()
      .filter((filePath) =>
        /\buse(?:Query|Mutation|InfiniteQuery|Queries)\s*\(/u.test(readProjectFile(filePath)),
      )
      .map(toProjectPath)
      .filter((relativePath) =>
        QUERY_HOOK_ALLOWED_PATH_PATTERNS.every((pattern) => !pattern.test(relativePath)),
      );

    expect(offenders).toEqual([]);
  });

  it("keeps active frontend source free of rounded marketing styling", () => {
    const offenders = collectSourceFiles()
      .filter((filePath) => {
        const contents = readProjectFile(filePath);
        return (
          /\brounded(?:-[a-z0-9[\]/.%]+)?\b/u.test(contents) ||
          /\bborder-radius\s*:/u.test(contents) ||
          /\bhero\b/u.test(contents)
        );
      })
      .map(toProjectPath);

    expect(offenders).toEqual([]);
  });

  it("keeps top-level workbench pages free of redundant title headers", () => {
    const offenders = TOP_LEVEL_PAGE_PATHS.filter((relativePath) => {
      const contents = readProjectFile(path.join(projectRoot, relativePath));
      return contents.includes("AppToolbar") || contents.includes("<h1");
    });

    expect(offenders).toEqual([]);
  });

  it("prevents direct generated DTO imports outside the typed facade", () => {
    const offenders = collectSourceFiles()
      .filter((filePath) => !toProjectPath(filePath).startsWith("src/types/"))
      .filter((filePath) => readProjectFile(filePath).includes("/types/generated"))
      .map(toProjectPath);

    expect(offenders).toEqual([]);
  });

  it("keeps user-visible JSX text behind typed localization resources", () => {
    const offenders = collectSourceFiles()
      .filter((filePath) => filePath.endsWith(".tsx"))
      .flatMap((filePath) => findVisibleJsxLiterals(filePath));

    expect(offenders).toEqual([]);
  });

  it("documents stable interaction feedback and removes manual account probes", () => {
    const frontendArchitecture = readProjectFile(
      path.join(projectRoot, "..", "..", "docs/agents/frontend-architecture.md"),
    );
    const accountSource = [
      "src/features/settings/components/AccountSettingsSection.tsx",
      "src/features/settings/components/ApiKeyRow.tsx",
      "src/features/settings/components/ActiveSubscriptionPanel.tsx",
    ]
      .map((relativePath) => readProjectFile(path.join(projectRoot, relativePath)))
      .join("\n");

    expect(frontendArchitecture).toContain("## Interaction Feedback");
    expect(accountSource).not.toMatch(/probeKey|refreshSubscription|onProbe/u);
  });

  it("keeps frontend promise failures on the shared logging path", () => {
    const loggerPath = "src/app/logger.ts";
    const offenders = collectSourceFiles()
      .filter((filePath) => toProjectPath(filePath) !== loggerPath)
      .filter((filePath) =>
        /console\.(debug|info|warn|error|log)\s*\(/u.test(readProjectFile(filePath)),
      )
      .map(toProjectPath);
    const silentPromiseCatches = collectSourceFiles()
      .filter((filePath) =>
        /catch\s*\(\s*\)[^{]*\{?\s*(?:return\s+)?undefined/u.test(readProjectFile(filePath)),
      )
      .map(toProjectPath);
    const loggerSource = readProjectFile(path.join(projectRoot, loggerPath));

    expect(offenders).toEqual([]);
    expect(silentPromiseCatches).toEqual([]);
    expect(loggerSource).toContain("reportBackgroundPromise");
    expect(loggerSource).toContain("installGlobalErrorHandlers");
    expect(loggerSource).toContain("@tauri-apps/plugin-log");
  });

  it("keeps desktop host permissions available", () => {
    const capability: unknown = JSON.parse(
      readProjectFile(path.join(projectRoot, "src-tauri/capabilities/default.json")),
    );

    expect(hasStringPermissions(capability)).toBe(true);

    if (!hasStringPermissions(capability)) {
      throw new Error("Capability permissions must be a string array.");
    }

    expect(capability.permissions).toEqual(
      expect.arrayContaining([
        "core:window:allow-close",
        "core:window:allow-minimize",
        "core:window:allow-start-dragging",
        "core:window:allow-toggle-maximize",
        "log:default",
      ]),
    );
  });

  it("keeps production and development CSPs explicit", () => {
    const config: unknown = JSON.parse(
      readProjectFile(path.join(projectRoot, "src-tauri/tauri.conf.json")),
    );
    const app = hasProperty(config, "app") ? config.app : undefined;
    const security = hasProperty(app, "security") ? app.security : undefined;
    const csp = hasProperty(security, "csp") ? security.csp : undefined;
    const devCsp = hasProperty(security, "devCsp") ? security.devCsp : undefined;

    expect(hasCspDirectives(csp)).toBe(true);
    expect(hasCspDirectives(devCsp)).toBe(true);
    if (!hasCspDirectives(csp) || !hasCspDirectives(devCsp)) {
      throw new Error("Tauri CSP directives must be explicit objects.");
    }

    expect(csp["connect-src"]).toContain("ipc:");
    expect(csp["img-src"]).toContain("data:");
    expect(csp["object-src"]).toBe("'none'");
    expect(devCsp["connect-src"]).toContain("ws://localhost:5173");
  });
});

function findVisibleJsxLiterals(filePath: string): string[] {
  const source = ts.createSourceFile(
    filePath,
    readProjectFile(filePath),
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  const offenders: string[] = [];
  const visit = (node: ts.Node) => {
    if (ts.isJsxText(node)) {
      const value = node.text.replace(/\s+/gu, " ").trim();
      if (/[A-Za-z]/u.test(value) && !ALLOWED_LITERAL_LABELS.has(value)) {
        offenders.push(
          `${toProjectPath(filePath)}:${source.getLineAndCharacterOfPosition(node.pos).line + 1}:${value}`,
        );
      }
    }
    if (
      ts.isJsxAttribute(node) &&
      USER_VISIBLE_JSX_ATTRIBUTES.has(node.name.getText(source)) &&
      node.initializer &&
      ts.isStringLiteral(node.initializer)
    ) {
      const value = node.initializer.text.trim();
      if (/[A-Za-z]/u.test(value) && !ALLOWED_LITERAL_LABELS.has(value)) {
        offenders.push(
          `${toProjectPath(filePath)}:${source.getLineAndCharacterOfPosition(node.pos).line + 1}:${value}`,
        );
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  return offenders;
}
