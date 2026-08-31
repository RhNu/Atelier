import assert from "node:assert/strict";
import test from "node:test";
import { GitHub, verifiedSource } from "./github.mjs";

test("GitHub distinguishes 404 from permission, server and validation failures", async () => {
  const missing = new GitHub(
    "owner/repo",
    "unused",
    async () => new Response(null, { status: 404 }),
  );
  assert.equal(await missing.api("missing", { missing: true }), null);
  await assert.rejects(missing.api("missing"), /HTTP 404/);
  for (const status of [403, 422, 500]) {
    const github = new GitHub(
      "owner/repo",
      "unused",
      async () => new Response("failure", { status }),
    );
    await assert.rejects(github.api("releases", { missing: true }), new RegExp(`HTTP ${status}`));
  }
});

test("tag identity peels annotated tags and never moves an existing reference", async () => {
  const github = new GitHub("owner/repo", "unused");
  const calls = [];
  github.api = async (path, options) => {
    calls.push({ path, options });
    return path.startsWith("git/ref/")
      ? { object: { type: "tag", sha: "annotated" } }
      : { object: { type: "commit", sha: "source" } };
  };
  assert.equal(await github.tagSha("v1.0.0"), "source");
  await github.ensureTag("v1.0.0", "source");
  await assert.rejects(github.ensureTag("v1.0.0", "another"), /Refusing to move/);
  assert.ok(calls.every((call) => !call.options?.method));
});

test("CI gate uses the resolved target SHA, waits only for that main push, and rejects failures", async (t) => {
  const originalRef = process.env.GITHUB_REF;
  t.after(() => {
    if (originalRef === undefined) delete process.env.GITHUB_REF;
    else process.env.GITHUB_REF = originalRef;
  });
  process.env.GITHUB_REF = "refs/heads/main";
  const selectedSha = "a".repeat(40);
  const calls = [];
  let polls = 0;
  let conclusion = "success";
  const github = {
    requireAncestor: async (base, head) => assert.deepEqual([base, head], ["target", "main"]),
    api: async (path) => {
      calls.push(path);
      if (path.startsWith("commits/")) return { sha: "target" };
      return {
        workflow_runs: [
          {
            id: 12,
            head_sha: "target",
            head_branch: "main",
            event: "push",
            status: ++polls > 1 ? "completed" : "in_progress",
            conclusion,
            html_url: "run-url",
          },
        ],
      };
    },
  };
  let waits = 0;
  assert.deepEqual(
    await verifiedSource(github, selectedSha, {
      wait: async () => {
        waits++;
      },
      attempts: 3,
    }),
    { sha: "target", ci_run: 12 },
  );
  assert.equal(waits, 1);
  assert.match(calls[1], /head_sha=target&event=push&branch=main/);
  conclusion = "failure";
  await assert.rejects(verifiedSource(github, selectedSha), /CI did not pass/);
  github.api = async (path) =>
    path.startsWith("commits/") ? { sha: "target" } : { workflow_runs: [] };
  await assert.rejects(
    verifiedSource(github, selectedSha, { attempts: 1 }),
    /No successful main CI/,
  );
  await assert.rejects(verifiedSource(github, "main"), /full commit SHA/);
  process.env.GITHUB_REF = "refs/tags/v1.0.0";
  await assert.rejects(verifiedSource(github, "main"), /Run the release workflow from main/);
});

test("branch publication rejects source ancestry rollback and divergence", async () => {
  const github = new GitHub("owner/repo", "unused");
  for (const status of ["behind", "diverged"]) {
    github.api = async () => ({ status });
    await assert.rejects(github.requireAncestor("old", "new"), /rollback/);
  }
  github.api = async () => ({ status: "ahead" });
  await github.requireAncestor("old", "new");
});
