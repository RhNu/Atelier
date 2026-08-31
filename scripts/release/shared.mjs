import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { appendFile, lstat, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

export function run(command, args, options = {}) {
  const result = spawnSync(command, args, { stdio: "inherit", ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} failed (${result.status ?? result.signal})`);
  return result.stdout?.toString().trim();
}

export function stableVersion(value) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(value)) {
    throw new Error(`Expected stable SemVer, received: ${value}`);
  }
  return value;
}

export function compareVersions(left, right) {
  const a = stableVersion(left).split(".").map(BigInt);
  const b = stableVersion(right).split(".").map(BigInt);
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return a[i] > b[i] ? 1 : -1;
  }
  return 0;
}

export function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

export function repository() {
  const value = required("GITHUB_REPOSITORY");
  if (!/^[\w.-]+\/[\w.-]+$/.test(value)) throw new Error("Invalid GITHUB_REPOSITORY");
  return value;
}

export const readJson = async (path) => JSON.parse(await readFile(path, "utf8"));
export const json = (value) => `${JSON.stringify(value, null, 2)}\n`;
export function describeBytes(name, bytes) {
  return {
    name: assetName(name),
    size: Buffer.byteLength(bytes),
    digest: `sha256:${createHash("sha256").update(bytes).digest("hex")}`,
  };
}
export async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, json(value));
}

export async function outputs(values) {
  for (const [key, value] of Object.entries(values)) {
    if (/\r|\n/.test(String(value))) throw new Error(`Multiline workflow output: ${key}`);
    console.log(`${key}=${value}`);
    if (process.env.GITHUB_OUTPUT) await appendFile(process.env.GITHUB_OUTPUT, `${key}=${value}\n`);
  }
}

// Release attachments have a flat namespace. Reject paths that are ambiguous on Windows, too.
export function assetName(name) {
  if (
    !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(name) ||
    /[.]$/.test(name) ||
    /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(name)
  ) {
    throw new Error(`Unsafe release asset name: ${name}`);
  }
  return name;
}

export async function filesIn(root) {
  const entries = await readdir(root, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    if (entry.isDirectory()) {
      files.push(...(await filesIn(join(root, entry.name))).map((name) => `${entry.name}/${name}`));
    } else if (entry.isFile()) files.push(entry.name);
    else throw new Error(`Unsupported release file: ${join(root, entry.name)}`);
  }
  return files.sort();
}

export async function digestFile(path) {
  const stat = await lstat(path);
  if (!stat.isFile() || stat.size === 0) throw new Error(`Missing or empty artifact: ${path}`);
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return { size: stat.size, digest: `sha256:${hash.digest("hex")}` };
}

export async function describeFiles(directory, names) {
  return Promise.all(
    names.map(async (name) => ({
      name: assetName(name),
      ...(await digestFile(join(directory, name))),
    })),
  );
}

export async function verifyFiles(directory, assets) {
  const names = assets.map((asset) => assetName(asset.name));
  if (new Set(names).size !== names.length) throw new Error("Duplicate release assets");
  for (const asset of assets) {
    const actual = await digestFile(join(directory, asset.name));
    if (actual.size !== asset.size || actual.digest !== asset.digest) {
      throw new Error(`Artifact integrity mismatch: ${asset.name}`);
    }
  }
}

export function main(moduleUrl, callback) {
  if (process.argv[1] && pathToFileURL(process.argv[1]).href === moduleUrl) {
    callback().catch((error) => {
      console.error(error.message);
      process.exitCode = 1;
    });
  }
}
