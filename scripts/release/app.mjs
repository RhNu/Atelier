import { copyFile, mkdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { GitHub, verifiedSource } from "./github.mjs";
import { publishRelease } from "./publish.mjs";
import {
  compareVersions,
  describeFiles,
  filesIn,
  main,
  outputs,
  readJson,
  repository,
  required,
  run,
  stableVersion,
  writeJson,
} from "./shared.mjs";

export function updaterManifest(plan, installer, signature) {
  if (!signature.trim()) throw new Error("Updater signature is empty");
  return {
    version: plan.version,
    notes: plan.notes,
    pub_date: plan.created_at,
    platforms: {
      "windows-x86_64": {
        signature: signature.trim(),
        url: `https://github.com/${plan.repository}/releases/download/${plan.tag}/${encodeURIComponent(installer)}`,
      },
    },
  };
}

export async function requireCurrentVersion(github, version) {
  const applications = (await github.releases()).filter(
    (release) => !release.draft && !release.prerelease && /^v\d+\.\d+\.\d+$/.test(release.tag_name),
  );
  if (applications.some((release) => compareVersions(release.tag_name.slice(1), version) > 0)) {
    throw new Error(`Refusing to publish ${version} behind an existing application release`);
  }
  return applications.sort((a, b) => compareVersions(b.tag_name.slice(1), a.tag_name.slice(1)))[0];
}

export async function prepareApp(github, ref) {
  const source = await verifiedSource(github, ref);
  const manifest = await github.content("apps/desktop/package.json", source.sha);
  const version = stableVersion(manifest.version);
  const tag = `v${version}`;
  const tagSha = await github.tagSha(tag);
  if (tagSha && tagSha !== source.sha)
    throw new Error(`Tag ${tag} already points to another commit`);
  const previous = await requireCurrentVersion(github, version);
  if (previous?.tag_name === tag) throw new Error(`${tag} is already published; use a new version`);
  const changes = previous
    ? `https://github.com/${github.repository}/compare/${previous.tag_name}...${source.sha}`
    : `https://github.com/${github.repository}/commits/${source.sha}`;
  return {
    format: "atelier.application-release.v1",
    repository: github.repository,
    source_sha: source.sha,
    ci_run: source.ci_run,
    automation_sha: required("GITHUB_SHA"),
    tag,
    tag_sha: source.sha,
    version,
    notes: `Atelier ${version}\n\n[Full changelog](${changes})`,
    created_at: new Date().toISOString(),
  };
}

export async function configureApp(root, plan) {
  for (const secret of ["TAURI_SIGNING_PRIVATE_KEY", "TAURI_SIGNING_PRIVATE_KEY_PASSWORD"])
    required(secret);
  if ((await readJson(join(root, "apps/desktop/package.json"))).version !== plan.version) {
    throw new Error("Application version changed after preparation");
  }
  const sha = run("git", ["rev-parse", "HEAD"], { cwd: root, stdio: "pipe" });
  if (sha !== plan.source_sha) throw new Error("Build source does not match the prepared commit");
  await writeJson(join(root, "apps/desktop/src-tauri/tauri.release.conf.json"), {
    plugins: {
      updater: {
        endpoints: [`https://github.com/${plan.repository}/releases/latest/download/latest.json`],
      },
    },
  });
}

export async function packApp(root, plan, output) {
  const bundle = join(root, "target/x86_64-pc-windows-msvc/release/bundle/nsis");
  const installer = `Atelier_${plan.version}_x64-setup.exe`;
  const names = await filesIn(bundle);
  const installers = names.filter((name) => name.endsWith("-setup.exe"));
  if (installers.length !== 1 || installers[0] !== installer) {
    throw new Error(`Expected exactly ${installer}; found ${installers.join(", ")}`);
  }
  const signatureName = `${installer}.sig`;
  const signature = await readFile(join(bundle, signatureName), "utf8");
  await mkdir(output, { recursive: true });
  for (const name of [installer, signatureName])
    await copyFile(join(bundle, name), join(output, name));
  await writeJson(join(output, "latest.json"), updaterManifest(plan, installer, signature));
  await writeJson(join(output, "release.json"), {
    ...plan,
    assets: await describeFiles(output, [installer, signatureName, "latest.json"]),
  });
}

main(import.meta.url, async () => {
  const [command, first, second, third] = process.argv.slice(2);
  if (command === "prepare" && first) {
    const plan = await prepareApp(new GitHub(), process.env.SOURCE_REF || required("GITHUB_SHA"));
    await writeJson(first, plan);
    await outputs({ source_sha: plan.source_sha });
  } else if (command === "configure" && first && second) {
    await configureApp(first, await readJson(second));
  } else if (command === "pack" && first && second && third) {
    await packApp(first, await readJson(second), third);
  } else if (command === "publish" && first) {
    const plan = await readJson(join(first, "release.json"));
    if (
      plan.format !== "atelier.application-release.v1" ||
      plan.repository !== repository() ||
      plan.automation_sha !== required("GITHUB_SHA")
    )
      throw new Error("Untrusted application artifact");
    const github = new GitHub();
    await requireCurrentVersion(github, plan.version);
    await publishRelease(github, plan, first, { body: plan.notes, latest: true });
  } else
    throw new Error(
      "Usage: app.mjs prepare <plan> | configure <source> <plan> | pack <source> <plan> <output> | publish <artifact>",
    );
});
