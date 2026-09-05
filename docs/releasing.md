# Releasing Atelier

Releases are explicitly started from **Actions → Release application / Release resources → Run
workflow**, with the workflow branch set to `main`. The optional `commit` input accepts a full
40-character commit SHA already on main; blank uses the workflow's own main revision. Branch/tag
names and abbreviated SHAs are not accepted, so retries cannot drift to a newer commit. Pushing
a tag does not publish anything.

Both workflows freeze the source SHA and require successful `CI` for that exact main push. They
wait up to 20 minutes for an in-progress/missing run, fail on unsuccessful CI, and never substitute
the workflow's own SHA or a pull-request merge check. Workflow automation comes from the main
revision used to start the run; application/resource source comes from the selected SHA. Both are
recorded in the release artifact.

## Responsibilities

| Stage | Responsibility |
| --- | --- |
| Local | Prepare the version/content, commit, and push to main. |
| CI | Rust/frontend checks, source catalog domain validation, release-script tests and lint. |
| Prepare/build | Freeze release inputs; build/sign the app or stage the complete resource batch. |
| Publish | Verify the saved artifact, create/resume a draft, upload all attachments, then publish. |

`Run workflow` is the approval boundary. Workflows create missing version tags against the frozen
source; they never move existing tags. Stable SemVer is required. Application versions come only
from `apps/desktop/package.json`; resource versions come from `resources/catalog/catalog-v1.json`.

Release jobs do not rerun code checks already passed by their source CI. Tauri runs the frontend
production build exactly once through `beforeBuildCommand`. Root `pnpm fmt:check`, `pnpm lint`, and
`pnpm test` include the release tooling. `pnpm release:test` runs its isolated Node tests without
GitHub credentials, network access, or real publications.

## Application releases

The normal release path is a single local command. It prepares the version, creates and pushes the
dedicated release commit, waits for CI on that exact commit, dispatches the application workflow,
and follows it through publication:

```powershell
cargo xtask release patch
cargo xtask release minor
cargo xtask release major
cargo xtask release 0.6.0
```

The command requires a clean, synchronized `main` branch and authenticated GitHub CLI. It never
commits unrelated changes, moves tags, or replaces published assets. Use `--dry-run` to inspect the
resolved plan, `--yes` for non-interactive use, `--no-wait` to return after dispatch, and `--json`
for a machine-readable final result. An interrupted or failed command saves its checkpoint below
`.git/atelier/`; rerun the same selector to resume. A failed release run is resumed with
`gh run rerun --failed`, preserving the workflow's saved artifact and publication safeguards.

The lower-level preparation-only command remains available when manual orchestration is needed:

```powershell
cargo xtask release prepare 0.5.3
git diff -- apps/desktop/package.json
git add apps/desktop/package.json
git commit -m "chore(release): prepare 0.5.3"
git push origin main
```

Then run `Release application` from main. GitHub CLI remains an optional low-level alternative to
the Actions UI:

```powershell
gh workflow run release-app.yml --ref main
```

The Windows x64 build emits an NSIS installer, its Tauri updater `.sig`, and `latest.json` with a
`windows-x86_64` entry. These files plus `release.json` (source/automation SHAs, CI run, version,
release notes and asset SHA-256 values) are saved as the `application-release` Actions artifact
for 14 days. The separate publish job rechecks those bytes before upload. It grants `contents:
write` only to publishing, not to dependency installation, tests or signing.

Application release notes link to the comparison against the previous application version, not
resource tags. Only application releases become GitHub latest. Publication is serialized and
rejects older application versions, preventing a slow/retried build from moving latest backwards.

## Updater signing

Keep these GitHub Actions repository secrets configured:

- `TAURI_SIGNING_PRIVATE_KEY`: the complete multiline private key, not a path or an extra Base64
  encoding.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the exact nonempty password, without a trailing newline.

The existing local copies are `.secrets/tauri-updater/atelier-updater.key` and
`.secrets/tauri-updater/atelier-updater.password`. The whole `.secrets/` directory is ignored. Never
print, commit, upload, or cache these files. Retain the key securely outside the repository too:
losing it prevents installed applications from accepting future updates.

The public key in `apps/desktop/src-tauri/tauri.conf.json` must match the signing key. Tauri creates
the signature during packaging; publication checks its presence and exact artifact bytes, while
the updater cryptographically verifies it before installation. Updater signing is independent of
Windows Authenticode and GitHub release immutability. Rotate keys with an old-key-signed update
that embeds the new public key before publishing updates signed only by the new key.

## Resource releases and catalog

Update the source catalog with new resource versions, dependencies, file sizes, SHA-256 values,
and pinned HTTPS URLs. Include required license files and provenance with repository-owned
payloads. Hash the final publication bytes, including final line endings; do not change bytes
after computing metadata.

Repository-owned payloads live at `resources/payloads/<id>/<version>/` and remain managed by Git
LFS where configured. Release assets have a flat namespace: use unique, portable filenames, not
nested directories. URLs must exactly name the corresponding asset in
`resource-<id>-v<version>`. Fixed upstream resources need no local payload and are not probed during
release. Runtime downloads still enforce HTTPS, declared sizes and SHA-256.

```powershell
cargo xtask resource catalog
git add resources .gitattributes
git commit -m "chore(resources): prepare resource updates"
git push origin main
gh workflow run release-resources.yml --ref main
```

The workflow stages the **complete source catalog as one batch**. Already published resources with
matching descriptors and asset digests are reused without downloading/copying their payloads.
It selectively fetches only new or unfinished repository-owned LFS payloads and checks each
payload's exact file set and size/hash. It creates frozen `resource.json` descriptors and saves
the batch as `resource-release` for 14 days. Existing resource tags may refer to earlier main
commits only when their tagged descriptor matches exactly. Existing public releases are checked,
not overwritten.

Every resource release is non-latest. Only after all releases succeed does the publisher update
the `resource-catalog` **branch**, which contains only:

- `catalog-v1.json`: the complete downloadable catalog.
- `publication.json`: the source SHA that produced it.

The application embeds this stable endpoint at build time:

```text
https://raw.githubusercontent.com/<owner>/<repository>/refs/heads/resource-catalog/catalog-v1.json
```

The branch is created automatically on first resource publication; it is distinct from any
historical tag of the same name. No special catalog Release, mutable release attachment, external
hosting service, or extra deployment workflow is needed. The GitHub token needs permission to
create/update this branch; configure repository rules accordingly. Do not edit it manually.

Catalog updates reject source ancestry rollback/divergence, version downgrades, and same-version
descriptor changes. A single non-force branch update publishes the new catalog. Concurrent
writers cannot replace each other's commits. A failed batch leaves the previous catalog visible;
the successfully published subset can be reused by a retry. On initial setup, publish resources
before the application so its catalog endpoint is available.

## Failure recovery and immutable releases

- If preparation or CI fails, fix the source and start a new run.
- If build/staging fails, rerun failed jobs. Successful immutable Actions artifacts are retained.
- If upload/publication fails, use **Re-run failed jobs**. The publish job reuses the same artifact
  without recompiling, regenerating release notes or re-signing.
- Drafts can be resumed. Unknown draft attachments stop publication for inspection. An already
  published version must match every asset name, size and SHA-256; it is never automatically
  overwritten. Publish changed bytes as a new version.
- If a saved artifact expires, start a new run. Do not use a newly rebuilt installer to replace an
  already public version; signatures or installer bytes may differ between builds.
- GitHub commands and API calls retry transient network failures, including EOF, timeouts,
  connection resets, and 5xx responses, up to 10 times with a 3-second delay. Deterministic
  failures such as authentication, permission, validation, and unexpected 404 responses still
  terminate immediately; authentication/network errors never trigger release recreation.

Release immutability is currently disabled. The pipeline nevertheless uses draft → complete
attachments → publish, so enabling it later does not require changing the catalog design.
Disabling immutability does not unlock releases created while it was enabled or guarantee reuse
of a previously protected tag name. Never delete/recreate tags as a recovery strategy. See
[GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases).

## Cache ownership

- Node is pinned by `.node-version`; pnpm by root `package.json`; Rust by `rust-toolchain.toml`.
- CI uses a dependency-oriented `ci-dev` Rust cache and writes it only from main pushes.
- Application builds use a separate `app-release` cache containing release-profile dependency,
  build-script and fingerprint directories. Keys include platform, toolchain/lockfile/manifests/
  Cargo configuration and source SHA; restore prefixes reuse the preceding compatible snapshot.
- Manual main workflows share main's cache scope across versions. Cache saving completes in the
  build job, independent of later publishing. Installers, signatures and secrets are excluded.
- pnpm store caching remains enabled; no `node_modules`, large model payload, or remote URL
  availability cache is maintained.

Measure restore time and release compilation time, not just cache hits. Old cache entries expire
normally; the publication workflow does not delete caches or perform repository maintenance.
