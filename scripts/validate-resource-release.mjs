import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { lstat, open, readdir, readFile } from "node:fs/promises";
import { join, relative, resolve, sep } from "node:path";

const [tag] = process.argv.slice(2);
const repository = process.env.GITHUB_REPOSITORY;
if (!tag || !repository || !/^[^/]+\/[^/]+$/.test(repository)) {
  throw new Error("usage: validate-resource-release.mjs resource-<id>-v<version>");
}

const match = /^resource-(?<id>[a-z0-9][a-z0-9-]*)-v(?<version>\d+\.\d+\.\d+)$/.exec(tag);
if (!match) throw new Error(`invalid resource tag: ${tag}`);
const catalog = JSON.parse(
  (await readFile("resources/catalog/catalog-v1.json", "utf8")).replaceAll(
    "__GITHUB_REPOSITORY__",
    repository,
  ),
);
const resource = catalog.resources?.find((candidate) => candidate.id === match.groups.id);
if (!resource) throw new Error(`unknown resource: ${match.groups.id}`);
if (resource.version !== match.groups.version) {
  throw new Error(
    `resource tag version ${match.groups.version} does not match ${resource.id}@${resource.version}`,
  );
}
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/.test(resource.version)) {
  throw new Error(`invalid resource version: ${resource.version}`);
}
if (!Array.isArray(resource.files) || resource.files.length === 0) {
  throw new Error(`resource has no files: ${resource.id}`);
}

const files = new Map();
for (const file of resource.files) {
  if (
    typeof file.path !== "string" ||
    file.path.length === 0 ||
    file.path.startsWith("/") ||
    file.path.includes("\\") ||
    file.path.split("/").some((part) => part === ".." || part === ".") ||
    files.has(file.path)
  ) {
    throw new Error(`invalid or duplicate resource path: ${file.path}`);
  }
  if (
    !Number.isSafeInteger(file.size_bytes) ||
    file.size_bytes <= 0 ||
    typeof file.sha256 !== "string" ||
    !/^[0-9a-f]{64}$/i.test(file.sha256) ||
    !Array.isArray(file.urls) ||
    file.urls.length === 0 ||
    file.urls.some((url) => typeof url !== "string" || !url.startsWith("https://"))
  ) {
    throw new Error(`invalid file metadata: ${resource.id}/${file.path}`);
  }
  files.set(file.path, file);
}

const payloadRoot = resolve("resources/payloads", resource.id, resource.version);
const githubPayload = resource.files.some((file) =>
  file.urls.some((url) => url.includes(`/releases/download/resource-${resource.id}-v${resource.version}/`)),
);
let payloadExists = false;
try {
  payloadExists = (await lstat(payloadRoot)).isDirectory();
} catch (error) {
  if (error.code !== "ENOENT") throw error;
}
if (githubPayload && !payloadExists) {
  throw new Error(`missing repository-owned payload directory: ${payloadRoot}`);
}
if (payloadExists) {
  const payloadFiles = await listFiles(payloadRoot);
  if (payloadFiles.length !== files.size || payloadFiles.some((file) => !files.has(file))) {
    throw new Error(`payload file set does not match descriptor: ${payloadRoot}`);
  }
  for (const [path, file] of files) {
    const fullPath = resolve(payloadRoot, path);
    const contained = relative(payloadRoot, fullPath);
    if (!contained || contained.startsWith(`..${sep}`) || resolve(payloadRoot, contained) !== fullPath) {
      throw new Error(`payload path escapes its resource root: ${path}`);
    }
    const stat = await lstat(fullPath);
    if (!stat.isFile() || stat.size !== file.size_bytes) {
      throw new Error(`payload size mismatch: ${fullPath}`);
    }
    if (await isLfsPointer(fullPath)) {
      throw new Error(`payload is still an LFS pointer: ${fullPath}`);
    }
    const hash = await sha256(fullPath);
    if (hash !== file.sha256.toLowerCase()) {
      throw new Error(`payload hash mismatch: ${fullPath}`);
    }
  }
}
console.log(`Resource ${resource.id}@${resource.version} is valid.`);

async function listFiles(root) {
  const entries = await readdir(root, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const child = join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await listFiles(child)).map((file) => `${entry.name}/${file}`));
    }
    else if (entry.isFile()) files.push(entry.name);
    else throw new Error(`unsupported payload entry: ${child}`);
  }
  return files;
}

async function isLfsPointer(path) {
  const handle = await open(path, "r");
  try {
    const buffer = Buffer.alloc(128);
    const { bytesRead } = await handle.read(buffer, 0, buffer.length, 0);
    return buffer
      .subarray(0, bytesRead)
      .toString("utf8")
      .startsWith("version https://git-lfs.github.com/spec/v1");
  } finally {
    await handle.close();
  }
}

function sha256(path) {
  return new Promise((resolveHash, reject) => {
    const hash = createHash("sha256");
    const stream = createReadStream(path);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("error", reject);
    stream.on("end", () => resolveHash(hash.digest("hex")));
  });
}
