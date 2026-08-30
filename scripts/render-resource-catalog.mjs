import { readFile, writeFile } from "node:fs/promises";

const [output] = process.argv.slice(2);
const repository = process.env.GITHUB_REPOSITORY;
if (!output || !repository || !/^[^/]+\/[^/]+$/.test(repository)) {
  throw new Error("output and GITHUB_REPOSITORY are required");
}
const template = await readFile("resources/catalog/catalog-v1.json", "utf8");
await writeFile(output, template.replaceAll("__GITHUB_REPOSITORY__", repository));
