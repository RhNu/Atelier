import { readFile } from "node:fs/promises";

const [path = "resources/catalog/catalog-v1.json"] = process.argv.slice(2);
const catalog = JSON.parse(await readFile(path, "utf8"));
const urls = catalog.resources.flatMap((resource) => resource.files.flatMap((file) => file.urls));
for (const url of urls) {
  const response = await fetch(url, { headers: { Range: "bytes=0-0" }, redirect: "follow" });
  if (!response.ok) throw new Error(`catalog URL is not accessible (${response.status}): ${url}`);
  await response.body?.cancel();
}
console.log(`Verified ${urls.length} catalog URLs.`);
