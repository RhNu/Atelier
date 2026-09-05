import { spawn } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
import { repository, required } from "./shared.mjs";

const DEFAULT_RETRIES = 10;
const RETRY_DELAY_MS = 3_000;

function isRetryableNetworkError(error) {
  const message = String(error?.message ?? error).toLowerCase();
  return [
    "eof",
    "fetch failed",
    "timed out",
    "timeout",
    "connection aborted",
    "connection closed",
    "connection refused",
    "connection reset",
    "network is unreachable",
    "no such host",
    "temporary failure",
    "broken pipe",
    "tls",
    "transport error",
    "http 500",
    "http 502",
    "http 503",
    "http 504",
    "bad gateway",
    "service unavailable",
    "gateway timeout",
  ].some((marker) => message.includes(marker));
}

async function retryNetwork(label, operation, { retries, wait }) {
  for (let retry = 0; retry <= retries; retry++) {
    try {
      return await operation();
    } catch (error) {
      if (retry >= retries || !isRetryableNetworkError(error)) throw error;
      console.error(
        `${label} failed: ${error.message}; retrying in ${RETRY_DELAY_MS / 1000} seconds (${retry + 1}/${retries})`,
      );
      await wait(RETRY_DELAY_MS);
    }
  }
  throw new Error(`${label} exhausted its retry budget`);
}

function runGh(args) {
  return new Promise((resolve, reject) => {
    const child = spawn("gh", args, { stdio: ["ignore", "pipe", "pipe"] });
    let stderr = "";
    child.stdout.on("data", (chunk) => process.stdout.write(chunk));
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
      process.stderr.write(chunk);
    });
    child.once("error", reject);
    child.once("close", (status, signal) => {
      if (status === 0) {
        resolve();
      } else {
        reject(new Error(`gh ${args.join(" ")} failed (${status ?? signal}): ${stderr.trim()}`));
      }
    });
  });
}

export class GitHub {
  constructor(
    repo = repository(),
    token = required("GH_TOKEN"),
    request = fetch,
    { retries = DEFAULT_RETRIES, wait = delay, execute = runGh } = {},
  ) {
    this.repository = repo;
    this.token = token;
    this.request = request;
    this.retries = retries;
    this.wait = wait;
    this.execute = execute;
  }

  async api(path, { method = "GET", body, missing = false } = {}) {
    return retryNetwork(
      `GitHub ${method} ${path}`,
      async () => {
        const response = await this.request(
          `https://api.github.com/repos/${this.repository}/${path}`,
          {
            method,
            headers: {
              Accept: "application/vnd.github+json",
              Authorization: `Bearer ${this.token}`,
              "X-GitHub-Api-Version": "2026-03-10",
              "Content-Type": "application/json",
            },
            body: body === undefined ? undefined : JSON.stringify(body),
            signal: AbortSignal.timeout(60_000),
          },
        );
        if (missing && response.status === 404) return null;
        if (!response.ok) {
          throw new Error(
            `GitHub ${method} ${path}: HTTP ${response.status}: ${await response.text()}`,
          );
        }
        return response.status === 204 ? null : response.json();
      },
      { retries: this.retries, wait: this.wait },
    );
  }

  async releases() {
    const result = [];
    for (let page = 1; ; page++) {
      const items = await this.api(`releases?per_page=100&page=${page}`);
      result.push(...items);
      if (items.length < 100) return result;
    }
  }

  async content(path, ref) {
    const file = await this.api(`contents/${path}?ref=${encodeURIComponent(ref)}`);
    return JSON.parse(Buffer.from(file.content, "base64").toString("utf8"));
  }

  async tagSha(tag) {
    const ref = await this.api(`git/ref/tags/${encodeURIComponent(tag)}`, { missing: true });
    if (!ref) return null;
    let object = ref.object;
    for (let depth = 0; object.type === "tag" && depth < 8; depth++) {
      object = (await this.api(`git/tags/${object.sha}`)).object;
    }
    if (object.type !== "commit") throw new Error(`Tag ${tag} does not identify a commit`);
    return object.sha;
  }

  async ensureTag(tag, sha) {
    const existing = await this.tagSha(tag);
    if (existing && existing !== sha) throw new Error(`Refusing to move tag ${tag}`);
    if (!existing)
      await this.api("git/refs", {
        method: "POST",
        body: { ref: `refs/tags/${tag}`, sha },
      });
  }

  async requireAncestor(base, head) {
    const comparison = await this.api(`compare/${base}...${head}`);
    if (!["ahead", "identical"].includes(comparison.status)) {
      throw new Error(`Refusing source rollback/divergence: ${base} -> ${head}`);
    }
  }

  async upload(tag, paths) {
    await retryNetwork(
      `gh release upload ${tag}`,
      () =>
        this.execute(["release", "upload", tag, ...paths, "--repo", this.repository, "--clobber"]),
      { retries: this.retries, wait: this.wait },
    );
  }
}

export async function verifiedSource(github, ref, { wait = delay, attempts = 80 } = {}) {
  if (process.env.GITHUB_REF !== "refs/heads/main") {
    throw new Error("Run the release workflow from main, not a tag or another branch");
  }
  if (!/^[0-9a-f]{40}$/.test(ref))
    throw new Error("Select a full commit SHA, or leave the input blank");
  const source = await github.api(`commits/${encodeURIComponent(ref)}`);
  await github.requireAncestor(source.sha, "main");
  for (let attempt = 0; attempt < attempts; attempt++) {
    const response = await github.api(
      `actions/workflows/ci.yml/runs?head_sha=${source.sha}&event=push&branch=main&per_page=1`,
    );
    const run = response.workflow_runs[0];
    if (run?.status === "completed") {
      if (run.conclusion !== "success") throw new Error(`CI did not pass: ${run.html_url}`);
      if (run.head_sha !== source.sha || run.event !== "push" || run.head_branch !== "main") {
        throw new Error("CI source identity mismatch");
      }
      return { sha: source.sha, ci_run: run.id };
    }
    console.log(`Waiting for main CI for ${source.sha} (${attempt + 1}/${attempts})`);
    if (attempt + 1 < attempts) await wait(15_000);
  }
  throw new Error(`No successful main CI for ${source.sha}; run CI before releasing`);
}
