import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { assertAssets, publishRelease } from "./publish.mjs";
import { describeFiles } from "./shared.mjs";

async function fixture(t) {
  const directory = await mkdtemp(join(tmpdir(), "atelier-publish-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  await writeFile(join(directory, "file.bin"), "payload");
  const assets = await describeFiles(directory, ["file.bin"]);
  const plan = { tag: "v1.0.0", tag_sha: "source", assets };
  const calls = [];
  let release;
  const github = {
    ensureTag: async () => {
      calls.push("tag");
    },
    tagSha: async () => "source",
    releases: async () => (release ? [release] : []),
    upload: () => {
      calls.push("upload");
      release.assets = assets.map((asset) => ({ ...asset, state: "uploaded" }));
    },
    api: async (_path, options = {}) => {
      calls.push(options.method || "GET");
      if (options.method === "POST")
        release = { id: 1, tag_name: plan.tag, draft: true, assets: [] };
      if (options.method === "PATCH") release = { ...release, ...options.body };
      return release;
    },
  };
  return {
    github,
    plan,
    directory,
    calls,
    get release() {
      return release;
    },
  };
}

test("all assets are uploaded and verified before publishing; a completed rerun is read-only", async (t) => {
  const f = await fixture(t);
  await publishRelease(f.github, f.plan, f.directory, { latest: true });
  assert.deepEqual(f.calls, ["tag", "POST", "upload", "GET", "PATCH"]);
  assert.equal(f.release.draft, false);
  assert.equal(f.release.make_latest, "true");
  f.calls.length = 0;
  await publishRelease(f.github, f.plan, f.directory);
  assert.deepEqual(f.calls, ["tag"]);
});

test("failed uploads never publish; a draft retry resumes without recreating the release", async (t) => {
  const f = await fixture(t);
  const upload = f.github.upload;
  f.github.upload = () => {
    throw new Error("upload failed");
  };
  await assert.rejects(publishRelease(f.github, f.plan, f.directory), /upload failed/);
  assert.equal(f.release.draft, true);
  assert.ok(!f.calls.includes("PATCH"));
  f.calls.length = 0;
  f.github.upload = upload;
  await publishRelease(f.github, f.plan, f.directory);
  assert.deepEqual(f.calls, ["tag", "upload", "GET", "PATCH"]);
  assert.equal(f.release.make_latest, "false");
});

test("incomplete uploads and moved tags cannot become public", async (t) => {
  const f = await fixture(t);
  f.github.upload = () => {};
  await assert.rejects(publishRelease(f.github, f.plan, f.directory), /incomplete/);
  assert.equal(f.release.draft, true);
  f.github.upload = () => {
    f.release.assets = f.plan.assets.map((a) => ({ ...a, state: "uploaded" }));
  };
  f.github.tagSha = async () => "moved";
  await assert.rejects(publishRelease(f.github, f.plan, f.directory), /tag changed/);
  assert.equal(f.release.draft, true);
});

test("asset validation compares names, state, sizes and digests, not just file existence", () => {
  const expected = [{ name: "a", size: 3, digest: "sha256:abc" }];
  for (const actual of [
    [],
    [{ ...expected[0], state: "starter" }],
    [{ ...expected[0], state: "uploaded", digest: "sha256:def" }],
  ]) {
    assert.throws(() => assertAssets(actual, expected));
  }
});

test("unchanged resources need no local payload and cannot be silently republished after deletion", async (t) => {
  const f = await fixture(t);
  await publishRelease(f.github, f.plan, f.directory);
  f.calls.length = 0;
  await rm(join(f.directory, "file.bin"));
  await publishRelease(f.github, { ...f.plan, published: true }, f.directory);
  assert.deepEqual(f.calls, []);
  f.github.releases = async () => [];
  await assert.rejects(
    publishRelease(f.github, { ...f.plan, published: true }, f.directory),
    /rerun preparation/,
  );
});
