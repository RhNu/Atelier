import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { descriptor, publishCatalog, renderCatalog } from "./catalog.mjs";
import {
  assertCatalogUpgrade,
  ownedFiles,
  packResources,
  publishResources,
  stagePayload,
} from "./resources.mjs";
import { digestFile, readJson, writeJson } from "./shared.mjs";

const resource = {
  id: "lexicon-core",
  version: "1.0.0",
  contract_version: 1,
  dependencies: [],
  files: [
    {
      path: "data.bin",
      size_bytes: 7,
      sha256: "0".repeat(64),
      urls: [
        "https://github.com/owner/repo/releases/download/resource-lexicon-core-v1.0.0/data.bin",
      ],
    },
  ],
};
const catalog = { catalog_version: "1.0.0", resources: [resource], groups: [] };

test("complete batch staging records provenance, fetches only owned LFS and checks existing tagged descriptors", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "atelier-resource-batch-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const oldSha = process.env.GITHUB_SHA;
  process.env.GITHUB_SHA = "automation";
  t.after(() => {
    if (oldSha === undefined) delete process.env.GITHUB_SHA;
    else process.env.GITHUB_SHA = oldSha;
  });
  const payload = join(root, "resources/payloads/lexicon-core/1.0.0");
  await mkdir(payload, { recursive: true });
  await writeFile(join(payload, "data.bin"), "payload");
  const current = structuredClone(catalog);
  current.resources[0].files[0].sha256 = (await digestFile(join(payload, "data.bin"))).digest.slice(
    7,
  );
  await writeJson(join(root, "resources/catalog/catalog-v1.json"), current);
  const tagged = structuredClone(current);
  const github = {
    repository: "owner/repo",
    api: async () => null,
    releases: async () => [],
    tagSha: async () => "original-tag-source",
    content: async () => tagged,
  };
  const commands = [];
  const pull = (...args) => {
    commands.push(args);
  };
  const output = join(root, "artifact");
  await packResources(github, root, { sha: "new-source", ci_run: 9 }, output, pull);
  assert.deepEqual(commands[0][1], [
    "lfs",
    "pull",
    "--include=resources/payloads/lexicon-core/1.0.0/**",
    "--exclude=",
  ]);
  const result = await readJson(join(output, "release.json"));
  assert.equal(result.source_sha, "new-source");
  assert.equal(result.automation_sha, "automation");
  assert.equal(result.releases[0].tag_sha, "original-tag-source");
  assert.equal(result.releases[0].assets.length, 2);
  github.releases = async () => [
    {
      tag_name: result.releases[0].tag,
      draft: false,
      assets: result.releases[0].assets.map((asset) => ({ ...asset, state: "uploaded" })),
    },
  ];
  commands.length = 0;
  await packResources(github, root, { sha: "new-source", ci_run: 9 }, output, pull);
  assert.equal(commands.length, 0);
  assert.equal((await readJson(join(output, "release.json"))).releases[0].published, true);
  tagged.resources[0].files[0].sha256 = "0".repeat(64);
  await assert.rejects(
    packResources(github, root, { sha: "new-source" }, output, pull),
    /different resource bytes/,
  );
});

test("catalog substitution does not change its contract and resource ownership is repository-specific", () => {
  assert.equal(ownedFiles(resource, "owner/repo").length, 1);
  assert.equal(ownedFiles(resource, "other/repo").length, 0);
  assert.deepEqual(descriptor(resource), {
    format: "atelier.downloadable-resource",
    schema_version: 1,
    ...resource,
  });
  assert.deepEqual(
    renderCatalog({ url: "https://github.com/__GITHUB_REPOSITORY__/x" }, "owner/repo"),
    { url: "https://github.com/owner/repo/x" },
  );
});

test("catalog rejects resource downgrades and same-version descriptor mutation", () => {
  assertCatalogUpgrade(catalog, structuredClone(catalog));
  const changed = structuredClone(catalog);
  changed.resources[0].files[0].sha256 = "1".repeat(64);
  assert.throws(() => assertCatalogUpgrade(catalog, changed), /without a new version/);
  changed.resources[0].version = "0.9.0";
  assert.throws(() => assertCatalogUpgrade(catalog, changed), /rolled back/);
  changed.resources[0].version = "1.1.0";
  assertCatalogUpgrade(catalog, changed);
});

test("payload staging checks exact file set and bytes, rejects LFS pointers and path traversal", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "atelier-resources-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const output = join(root, "artifact");
  const payload = join(root, "resources/payloads/lexicon-core/1.0.0");
  await mkdir(output);
  await assert.rejects(stagePayload(root, resource, "owner/repo", output), /Missing real payload/);
  await mkdir(payload, { recursive: true });
  const file = join(payload, "data.bin");
  await writeFile(file, "payload");
  const value = structuredClone(resource);
  value.files[0].sha256 = (await digestFile(file)).digest.slice(7);
  assert.equal((await stagePayload(root, value, "owner/repo", output)).length, 1);
  await writeFile(join(payload, "extra.txt"), "extra");
  await assert.rejects(stagePayload(root, value, "owner/repo", output), /file set mismatch/);
  await rm(join(payload, "extra.txt"));
  await writeFile(file, "version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 7\n");
  await assert.rejects(stagePayload(root, value, "owner/repo", output), /unresolved LFS pointer/);
  value.files[0].path = "../data.bin";
  await assert.rejects(stagePayload(root, value, "owner/repo", output), /Unsafe release asset/);
});

test("upstream resources do not need a local payload or network probe", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "atelier-upstream-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const value = structuredClone(resource);
  value.files[0].urls = ["https://huggingface.co/model/resolve/pinned/data.bin"];
  assert.deepEqual(await stagePayload(root, value, "owner/repo", root), []);
});

test("catalog commit contains only publication data and uses non-force fast-forward updates", async () => {
  const calls = [];
  const github = {
    api: async (path, options) => {
      calls.push({ path, ...options });
      return { sha: path === "git/trees" ? "tree" : "new-head" };
    },
  };
  const previous = { sha: "old-head", publication: { source_sha: "old-source" }, catalog };
  await publishCatalog(github, catalog, "new-source", previous);
  assert.deepEqual(
    calls[0].body.tree.map((item) => item.path),
    ["catalog-v1.json", "publication.json"],
  );
  assert.deepEqual(calls[1].body.parents, ["old-head"]);
  assert.deepEqual(calls[2], {
    path: "git/refs/heads/resource-catalog",
    method: "PATCH",
    body: { sha: "new-head", force: false },
  });
  calls.length = 0;
  await publishCatalog(github, catalog, "old-source", previous);
  assert.equal(calls.length, 0);
});

test("initial catalog publication creates an isolated branch without an application/latest release", async () => {
  const calls = [];
  const github = {
    api: async (path, options) => {
      calls.push({ path, ...options });
      return { sha: "new" };
    },
  };
  await publishCatalog(github, catalog, "source", null);
  assert.deepEqual(calls[1].body.parents, []);
  assert.deepEqual(calls[2], {
    path: "git/refs",
    method: "POST",
    body: { ref: "refs/heads/resource-catalog", sha: "new" },
  });
});

test("resource failure does not update the discoverable catalog", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "atelier-batch-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const calls = [];
  const github = {
    api: async (path) => {
      calls.push(path);
      return null;
    },
  };
  const plan = {
    source_sha: "source",
    catalog,
    releases: [
      { tag: "resource-test-v1.0.0", tag_sha: "source", assets: [{ name: "missing.bin" }] },
    ],
  };
  await assert.rejects(publishResources(github, plan, root));
  assert.deepEqual(calls, ["git/ref/heads/resource-catalog"]);
});
