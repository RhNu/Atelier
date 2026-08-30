import { readFile, writeFile } from "node:fs/promises";

const [id, output] = process.argv.slice(2);
if (!id || !output) throw new Error("usage: write-resource-descriptor.mjs <id> <output>");
const repository = process.env.GITHUB_REPOSITORY;
if (!repository) throw new Error("GITHUB_REPOSITORY is required");
const source = (await readFile("resources/catalog/catalog-v1.json", "utf8")).replaceAll(
  "__GITHUB_REPOSITORY__",
  repository,
);
const catalog = JSON.parse(source);
const resource = catalog.resources.find((candidate) => candidate.id === id);
if (!resource) throw new Error(`unknown resource: ${id}`);
await writeFile(
  output,
  `${JSON.stringify(
    { format: "atelier.downloadable-resource", schema_version: 1, ...resource },
    null,
    2,
  )}\n`,
);
