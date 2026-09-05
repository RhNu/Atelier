import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const workflow = (name) =>
  readFile(new URL(`../../.github/workflows/${name}`, import.meta.url), "utf8");

test("release workflows are manual, trusted-main only, queued and split before publication", async () => {
  for (const filename of ["release-app.yml", "release-resources.yml"]) {
    const source = await workflow(filename);
    assert.match(source, /workflow_dispatch:/);
    assert.doesNotMatch(source, /\bpush:|\btags:/);
    assert.match(source, /github.ref == 'refs\/heads\/main'/);
    assert.match(source, /queue: max/);
    assert.match(source, /actions\/upload-artifact@/);
    assert.match(source, /actions\/download-artifact@/);
    assert.match(source, /  publish:\n    needs: (build|prepare)/);
    assert.doesNotMatch(source, /cargo (test|clippy|fmt)|pnpm (test|lint|build)/);
  }
});

test("application publishing uses a separate release-profile cache and mutable branch catalog", async () => {
  const source = await workflow("release-app.yml");
  assert.match(source, /run-name: Release application .* inputs\.request_id/);
  assert.match(source, /request_id:/);
  assert.match(source, /app-release-\$\{\{ runner.os \}\}/);
  assert.match(source, /working-directory: source\/apps\/desktop/);
  assert.match(source, /node node_modules\/@tauri-apps\/cli\/tauri.js build --ci .* -- --locked/);
  assert.doesNotMatch(source, /pnpm .*exec tauri/);
  assert.match(
    source,
    /raw.githubusercontent.com\/\$\{\{ github.repository \}\}\/refs\/heads\/resource-catalog\/catalog-v1.json/,
  );
  assert.doesNotMatch(
    source,
    /releases\/download\/resource-catalog|GITHUB_REF_NAME|shared-key: atelier/,
  );
});
