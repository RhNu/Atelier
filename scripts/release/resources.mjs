import { copyFile, lstat, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { catalogState, descriptor, publishCatalog, renderCatalog } from "./catalog.mjs";
import { GitHub, verifiedSource } from "./github.mjs";
import { assertAssets, publishRelease } from "./publish.mjs";
import {
  assetName,
  compareVersions,
  describeBytes,
  digestFile,
  filesIn,
  json,
  main,
  outputs,
  readJson,
  repository,
  required,
  run,
  stableVersion,
  writeJson,
} from "./shared.mjs";

export function resourceTag(resource) {
  if (!/^[a-z0-9][a-z0-9-]*$/.test(resource.id))
    throw new Error(`Invalid resource id: ${resource.id}`);
  return `resource-${resource.id}-v${stableVersion(resource.version)}`;
}

export function ownedFiles(resource, repo) {
  const prefix = `https://github.com/${repo}/releases/download/${resourceTag(resource)}/`;
  return resource.files.filter((file) => file.urls.some((url) => url.startsWith(prefix)));
}

export function assertCatalogUpgrade(previous, next) {
  if (!previous) return;
  if (compareVersions(next.catalog_version, previous.catalog_version) < 0)
    throw new Error("Catalog version rollback");
  for (const resource of next.resources) {
    const old = previous.resources.find((item) => item.id === resource.id);
    if (!old) continue;
    const order = compareVersions(resource.version, old.version);
    if (order < 0 || (order === 0 && json(descriptor(resource)) !== json(descriptor(old)))) {
      throw new Error(`Resource ${resource.id} changed without a new version, or rolled back`);
    }
  }
}

export async function stagePayload(root, resource, repo, output) {
  const files = ownedFiles(resource, repo);
  const payload = join(root, "resources/payloads", resource.id, resource.version);
  const exists = await lstat(payload).catch((error) => {
    if (error.code === "ENOENT") return null;
    throw error;
  });
  if (!files.length) {
    if (exists) throw new Error(`Unexpected local payload for upstream resource ${resource.id}`);
    return [];
  }
  if (!exists?.isDirectory() || exists.isSymbolicLink())
    throw new Error(`Missing real payload: ${payload}`);
  const names = files.map((file) => assetName(file.path)).sort();
  if (
    new Set(names.map((name) => name.toLowerCase())).size !== names.length ||
    names.some((name) => name.toLowerCase() === "resource.json")
  ) {
    throw new Error(`Conflicting release asset names in ${resource.id}`);
  }
  if (json(await filesIn(payload)) !== json(names))
    throw new Error(`Payload file set mismatch: ${resource.id}`);
  const assets = [];
  for (const file of files) {
    const name = assetName(file.path);
    const expectedUrl = `https://github.com/${repo}/releases/download/${resourceTag(resource)}/${encodeURIComponent(name)}`;
    if (!file.urls.includes(expectedUrl))
      throw new Error(`Payload URL does not match asset name: ${name}`);
    const actual = await digestFile(join(payload, name));
    if (
      actual.size !== file.size_bytes ||
      actual.digest !== `sha256:${file.sha256.toLowerCase()}`
    ) {
      throw new Error(
        `Payload size/hash mismatch (or unresolved LFS pointer): ${resource.id}/${name}`,
      );
    }
    await copyFile(join(payload, name), join(output, name));
    assets.push({ name, ...actual });
  }
  return assets;
}

export async function packResources(github, root, source, output, pull = run) {
  const catalog = renderCatalog(
    await readJson(join(root, "resources/catalog/catalog-v1.json")),
    github.repository,
  );
  const previous = await catalogState(github);
  if (previous) await github.requireAncestor(previous.publication.source_sha, source.sha);
  assertCatalogUpgrade(previous?.catalog, catalog);
  const published = await github.releases();
  const releases = [];
  for (const resource of catalog.resources) {
    const tag = resourceTag(resource);
    const tagSha = await github.tagSha(tag);
    if (tagSha && tagSha !== source.sha) {
      const tagged = renderCatalog(
        await github.content("resources/catalog/catalog-v1.json", tagSha),
        github.repository,
      );
      const original = tagged.resources.find((item) => item.id === resource.id);
      if (!original || json(descriptor(original)) !== json(descriptor(resource))) {
        throw new Error(`Existing tag ${tag} describes different resource bytes`);
      }
    }
    const existing = published.find((entry) => entry.tag_name === tag && !entry.draft);
    const assets = [
      ...ownedFiles(resource, github.repository).map((file) => ({
        name: assetName(file.path),
        size: file.size_bytes,
        digest: `sha256:${file.sha256.toLowerCase()}`,
      })),
      describeBytes("resource.json", json(descriptor(resource))),
    ];
    if (existing) {
      if (!tagSha) throw new Error(`Published resource tag is missing: ${tag}`);
      assertAssets(existing.assets, assets);
    }
    releases.push({ tag, tag_sha: tagSha || source.sha, published: Boolean(existing), assets });
  }
  const pending = catalog.resources.filter((_resource, index) => !releases[index].published);
  const includes = pending
    .filter((item) => ownedFiles(item, github.repository).length)
    .map((item) => `resources/payloads/${item.id}/${item.version}/**`);
  if (includes.length)
    pull("git", ["lfs", "pull", `--include=${includes.join(",")}`, "--exclude="], { cwd: root });
  for (const resource of pending) {
    const tag = resourceTag(resource);
    const directory = join(output, tag);
    await mkdir(directory, { recursive: true });
    await stagePayload(root, resource, github.repository, directory);
    await writeJson(join(directory, "resource.json"), descriptor(resource));
  }
  await writeJson(join(output, "release.json"), {
    format: "atelier.resource-release.v1",
    repository: github.repository,
    source_sha: source.sha,
    ci_run: source.ci_run,
    automation_sha: required("GITHUB_SHA"),
    catalog,
    releases,
  });
}

export async function publishResources(github, plan, directory) {
  const previous = await catalogState(github);
  if (previous) await github.requireAncestor(previous.publication.source_sha, plan.source_sha);
  assertCatalogUpgrade(previous?.catalog, plan.catalog);
  for (const release of plan.releases) {
    await publishRelease(github, release, join(directory, release.tag), {
      body: `Frozen Atelier resource descriptor and payload. Source: ${release.tag_sha}`,
    });
  }
  // Only the completed batch becomes discoverable. Failed/retried uploads leave the old catalog intact.
  await publishCatalog(github, plan.catalog, plan.source_sha, previous);
}

main(import.meta.url, async () => {
  const [command, first, second, third] = process.argv.slice(2);
  const github = new GitHub();
  if (command === "prepare" && first) {
    const source = await verifiedSource(github, process.env.SOURCE_REF || required("GITHUB_SHA"));
    await writeJson(first, source);
    await outputs({ source_sha: source.sha });
  } else if (command === "pack" && first && second && third) {
    const source = await readJson(second);
    if (run("git", ["rev-parse", "HEAD"], { cwd: first, stdio: "pipe" }) !== source.sha) {
      throw new Error("Resource checkout does not match prepared source");
    }
    await packResources(github, first, source, third);
  } else if (command === "publish" && first) {
    const plan = await readJson(join(first, "release.json"));
    if (
      plan.format !== "atelier.resource-release.v1" ||
      plan.repository !== repository() ||
      plan.automation_sha !== required("GITHUB_SHA")
    )
      throw new Error("Untrusted resource artifact");
    await publishResources(github, plan, first);
  } else
    throw new Error(
      "Usage: resources.mjs prepare <plan> | pack <source> <plan> <output> | publish <artifact>",
    );
});
