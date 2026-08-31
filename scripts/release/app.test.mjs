import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { packApp, prepareApp, requireCurrentVersion, updaterManifest } from "./app.mjs";
import { readJson } from "./shared.mjs";

const plan = {
  version: "1.2.3",
  tag: "v1.2.3",
  repository: "owner/repo",
  source_sha: "source",
  notes: "Release notes",
  created_at: "2026-08-31T00:00:00Z",
};

test("application preparation is read-only and binds version, CI and notes to the selected source", async (t) => {
  const oldRef = process.env.GITHUB_REF;
  const oldSha = process.env.GITHUB_SHA;
  process.env.GITHUB_REF = "refs/heads/main";
  process.env.GITHUB_SHA = "b".repeat(40);
  t.after(() => {
    if (oldRef === undefined) delete process.env.GITHUB_REF;
    else process.env.GITHUB_REF = oldRef;
    if (oldSha === undefined) delete process.env.GITHUB_SHA;
    else process.env.GITHUB_SHA = oldSha;
  });
  const sha = "a".repeat(40);
  const github = {
    repository: "owner/repo",
    api: async (path, options) => {
      assert.equal(options?.method, undefined);
      if (path.startsWith("commits/")) return { sha };
      return {
        workflow_runs: [
          {
            id: 5,
            head_sha: sha,
            head_branch: "main",
            event: "push",
            status: "completed",
            conclusion: "success",
          },
        ],
      };
    },
    requireAncestor: async () => {},
    content: async (_path, ref) => {
      assert.equal(ref, sha);
      return { version: "1.2.3" };
    },
    tagSha: async () => null,
    releases: async () => [{ tag_name: "v1.2.2", draft: false }],
  };
  const result = await prepareApp(github, sha);
  assert.equal(result.source_sha, sha);
  assert.equal(result.tag_sha, sha);
  assert.equal(result.ci_run, 5);
  assert.equal(result.automation_sha, process.env.GITHUB_SHA);
  assert.match(result.notes, /compare\/v1.2.2\.\.\.aaaa/);
  github.tagSha = async () => "other";
  await assert.rejects(prepareApp(github, sha), /another commit/);
});

test("updater manifest is driven by explicit release identity, not dispatch branch or wall clock", () => {
  const manifest = updaterManifest(plan, "Atelier_1.2.3_x64-setup.exe", " signature\n");
  assert.equal(manifest.version, "1.2.3");
  assert.equal(manifest.pub_date, plan.created_at);
  assert.equal(manifest.platforms["windows-x86_64"].signature, "signature");
  assert.equal(
    manifest.platforms["windows-x86_64"].url,
    "https://github.com/owner/repo/releases/download/v1.2.3/Atelier_1.2.3_x64-setup.exe",
  );
  assert.throws(() => updaterManifest(plan, "installer.exe", " \n"), /empty/);
});

test("only application releases participate in latest/version ordering", async () => {
  const github = {
    releases: async () => [
      { tag_name: "resource-catalog", draft: false },
      { tag_name: "v2.0.0", draft: true },
      { tag_name: "v1.10.0", draft: false },
      { tag_name: "v1.9.0", draft: false },
    ],
  };
  await assert.rejects(requireCurrentVersion(github, "1.9.1"), /behind/);
  assert.equal((await requireCurrentVersion(github, "1.11.0")).tag_name, "v1.10.0");
});

test("packaging stages exact-version installer, signature and manifest; stale installers fail", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "atelier-app-release-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const bundle = join(root, "target/x86_64-pc-windows-msvc/release/bundle/nsis");
  await mkdir(bundle, { recursive: true });
  await writeFile(join(bundle, "Atelier_1.2.3_x64-setup.exe"), "installer");
  await writeFile(join(bundle, "Atelier_1.2.3_x64-setup.exe.sig"), "signature");
  const output = join(root, "artifact");
  await packApp(root, plan, output);
  const result = await readJson(join(output, "release.json"));
  assert.equal(result.source_sha, "source");
  assert.equal(result.assets.length, 3);
  await writeFile(join(bundle, "Atelier_1.2.2_x64-setup.exe"), "stale");
  await assert.rejects(packApp(root, plan, output), /Expected exactly/);
});
