import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  assetName,
  compareVersions,
  describeFiles,
  run,
  stableVersion,
  verifyFiles,
} from "./shared.mjs";

test("stable versions and asset names reject unsafe or ambiguous inputs", () => {
  for (const value of ["01.2.3", "1.2", "v1.2.3", "1.2.3-beta", "1.2.3+build", "main"]) {
    assert.throws(() => stableVersion(value));
  }
  for (const name of [
    "../installer.exe",
    "C:installer.exe",
    "a/b",
    "a\\b",
    "-file",
    "file.",
    "NUL",
    "con.txt",
  ]) {
    assert.throws(() => assetName(name));
  }
  assert.equal(compareVersions("1.10.0", "1.9.9"), 1);
  assert.equal(compareVersions("1.2.3", "1.2.3"), 0);
  assert.equal(compareVersions("1.2.3", "2.0.0"), -1);
});

test("every external command failure is fatal, including missing executables", () => {
  assert.throws(
    () => run(process.execPath, ["-e", "process.exit(17)"], { stdio: "pipe" }),
    /failed \(17\)/,
  );
  assert.throws(() => run("atelier-missing-command", [], { stdio: "pipe" }));
});

test("artifact transport boundary rejects missing, changed and duplicate assets", async (t) => {
  const directory = await mkdtemp(join(tmpdir(), "atelier-release-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  await writeFile(join(directory, "installer.exe"), "signed application");
  const assets = await describeFiles(directory, ["installer.exe"]);
  await verifyFiles(directory, assets);
  await assert.rejects(verifyFiles(directory, [...assets, ...assets]), /Duplicate/);
  await writeFile(join(directory, "installer.exe"), "tampered application");
  await assert.rejects(verifyFiles(directory, assets), /integrity mismatch/);
  await assert.rejects(verifyFiles(directory, [{ ...assets[0], name: "missing.exe" }]));
});
