import { join } from "node:path";
import { verifyFiles } from "./shared.mjs";

export function assertAssets(actual, expected) {
  if (actual.length !== expected.length)
    throw new Error("Release asset set is incomplete or unexpected");
  for (const asset of expected) {
    const remote = actual.find((entry) => entry.name === asset.name);
    if (
      !remote ||
      remote.state !== "uploaded" ||
      remote.size !== asset.size ||
      remote.digest !== asset.digest
    ) {
      throw new Error(`Release asset mismatch: ${asset.name}`);
    }
  }
}

export async function publishRelease(github, plan, directory, { body, latest = false } = {}) {
  if (plan.published) {
    const existing = (await github.releases()).find((entry) => entry.tag_name === plan.tag);
    if (!existing || existing.draft || (await github.tagSha(plan.tag)) !== plan.tag_sha) {
      throw new Error(`Published release ${plan.tag} changed; rerun preparation`);
    }
    assertAssets(existing.assets, plan.assets);
    console.log(`${plan.tag} is unchanged; reused without downloading its payload`);
    return existing;
  }
  await verifyFiles(directory, plan.assets);
  await github.ensureTag(plan.tag, plan.tag_sha);
  // Listing also finds drafts whose tag lookup endpoint can return 404.
  let release = (await github.releases()).find((entry) => entry.tag_name === plan.tag);
  if (release && !release.draft) {
    assertAssets(release.assets, plan.assets);
    console.log(`${plan.tag} is already complete; leaving published assets unchanged`);
  } else {
    if (!release)
      release = await github.api("releases", {
        method: "POST",
        body: {
          tag_name: plan.tag,
          target_commitish: plan.tag_sha,
          name: plan.tag,
          body,
          draft: true,
          prerelease: false,
          make_latest: "false",
        },
      });
    if (release.immutable) throw new Error(`Cannot resume immutable draft: ${plan.tag}`);
    const expectedNames = new Set(plan.assets.map((asset) => asset.name));
    if (release.assets.some((asset) => !expectedNames.has(asset.name))) {
      throw new Error(`Draft ${plan.tag} has unexpected assets; inspect it before retrying`);
    }
    github.upload(
      plan.tag,
      plan.assets.map((asset) => join(directory, asset.name)),
    );
    release = await github.api(`releases/${release.id}`);
    assertAssets(release.assets, plan.assets);
    // Recheck the tag immediately before making the release public.
    if ((await github.tagSha(plan.tag)) !== plan.tag_sha)
      throw new Error("Release tag changed during upload");
    release = await github.api(`releases/${release.id}`, {
      method: "PATCH",
      body: { draft: false, body, make_latest: latest ? "true" : "false" },
    });
  }
  return release;
}
