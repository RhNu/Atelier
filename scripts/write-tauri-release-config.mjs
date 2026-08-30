import { writeFile } from "node:fs/promises";

const [output] = process.argv.slice(2);
const repository = process.env.GITHUB_REPOSITORY;
if (!output || !repository || !/^[^/]+\/[^/]+$/.test(repository)) {
  throw new Error("usage: write-tauri-release-config.mjs <output>; GITHUB_REPOSITORY is required");
}
const config = {
  plugins: {
    updater: {
      endpoints: [`https://github.com/${repository}/releases/latest/download/latest.json`],
    },
  },
};
await writeFile(output, `${JSON.stringify(config, null, 2)}\n`);
