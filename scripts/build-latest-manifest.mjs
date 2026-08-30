import { readFile, writeFile } from "node:fs/promises";
import { basename } from "node:path";

const [artifact, signaturePath, output] = process.argv.slice(2);
const repository = process.env.GITHUB_REPOSITORY;
const tag = process.env.GITHUB_REF_NAME;
if (!artifact || !signaturePath || !output || !repository || !tag?.startsWith("v")) {
  throw new Error("artifact, signature, output, GITHUB_REPOSITORY and release tag are required");
}
const signature = (await readFile(signaturePath, "utf8")).trim();
const manifest = {
  version: tag.slice(1),
  notes: process.env.RELEASE_NOTES || `Atelier ${tag.slice(1)}`,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(basename(artifact))}`,
    },
  },
};
await writeFile(output, `${JSON.stringify(manifest, null, 2)}\n`);
