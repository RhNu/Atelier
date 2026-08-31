import { json } from "./shared.mjs";

export const catalogBranch = "resource-catalog";
export const renderCatalog = (catalog, repository) =>
  JSON.parse(JSON.stringify(catalog).replaceAll("__GITHUB_REPOSITORY__", repository));
export const descriptor = (resource) => ({
  format: "atelier.downloadable-resource",
  schema_version: 1,
  ...resource,
});

export async function catalogState(github) {
  const ref = await github.api(`git/ref/heads/${catalogBranch}`, { missing: true });
  if (!ref) return null;
  const sha = ref.object.sha;
  return {
    sha,
    publication: await github.content("publication.json", sha),
    catalog: await github.content("catalog-v1.json", sha),
  };
}

export async function publishCatalog(github, catalog, sourceSha, previous) {
  if (previous?.publication.source_sha === sourceSha && json(previous.catalog) === json(catalog)) {
    console.log("Catalog already published for this source");
    return;
  }
  const tree = await github.api("git/trees", {
    method: "POST",
    body: {
      tree: [
        { path: "catalog-v1.json", mode: "100644", type: "blob", content: json(catalog) },
        {
          path: "publication.json",
          mode: "100644",
          type: "blob",
          content: json({ source_sha: sourceSha }),
        },
      ],
    },
  });
  const commit = await github.api("git/commits", {
    method: "POST",
    body: {
      message: `chore(resources): publish catalog from ${sourceSha}`,
      tree: tree.sha,
      parents: previous ? [previous.sha] : [],
    },
  });
  // A concurrent writer produces a sibling commit, so force:false rejects it without losing data.
  if (previous)
    await github.api(`git/refs/heads/${catalogBranch}`, {
      method: "PATCH",
      body: { sha: commit.sha, force: false },
    });
  else
    await github.api("git/refs", {
      method: "POST",
      body: { ref: `refs/heads/${catalogBranch}`, sha: commit.sha },
    });
}
