import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

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
  "src/features/gallery",
  "src/features/settings",
  "src/routes",
] as const;

const REQUIRED_FRONTEND_DOCS = [
  "docs/agents/frontend-architecture.md",
  "docs/agents/frontend-workbench-foundation.md",
] as const;

const QUERY_HOOK_ALLOWED_PATH_PATTERNS = [
  /^src\/features\/[^/]+\/data\//u,
  /^src\/features\/[^/]+\/runtime\//u,
  /^src\/features\/workspace\//u,
  /^src\/platform\/atelier\//u,
] as const;

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

  it("prevents direct generated DTO imports outside the typed facade", () => {
    const offenders = collectSourceFiles()
      .filter((filePath) => !toProjectPath(filePath).startsWith("src/types/"))
      .filter((filePath) => readProjectFile(filePath).includes("/types/generated"))
      .map(toProjectPath);

    expect(offenders).toEqual([]);
  });

  it("keeps custom titlebar window permissions available", () => {
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
