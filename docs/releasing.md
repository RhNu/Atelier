# Release Guide

Atelier publishes application updates and downloadable resources from the same repository through
separate tag-triggered GitHub Actions workflows. Application releases may become the GitHub latest
release. Resource releases and the stable resource catalog never do.

## Tools and Responsibilities

- `cargo xtask` validates versions, descriptors, payloads, and creates local annotated tags.
- Git and Git LFS publish the release commit, tag, and repository-owned resource payloads.
- GitHub Actions builds and signs the application or publishes the selected resource after its tag
  is pushed.
- GitHub CLI is optional. Do not manually create releases that are owned by the workflows.

The local commands intentionally do not commit or push. The final explicit `git push` is the release
approval boundary.

## Tauri Updater Signing Material

The repository-local copies of the updater signing material live at:

```text
.secrets/tauri-updater/atelier-updater.key
.secrets/tauri-updater/atelier-updater.password
```

The complete `.secrets/` directory is ignored by Git. Never force-add it, print the private key in
logs, attach it to a release, or copy it into a workflow artifact. Keep another secure backup outside
the repository: losing the private key prevents installed applications from accepting future
updates.

Configure these GitHub Actions repository secrets before publishing an application release:

- `TAURI_SIGNING_PRIVATE_KEY`: the complete, multiline contents of
  `atelier-updater.key`, including its comment header. A filesystem path works for a local build but
  not for a hosted GitHub runner. Do not Base64-encode it because the current workflow does not
  decode it.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the exact password in
  `atelier-updater.password`, without quotes or an added trailing newline.

The public key is committed in `apps/desktop/src-tauri/tauri.conf.json` and is safe to distribute.
It must match the private key. Tauri updater signing is independent of Windows Authenticode signing.
Downloadable-resource releases do not use the Tauri private key; their files are verified through
the catalog SHA-256 values.

When rotating keys, first publish an update signed by the old key that embeds the new public key.
Only subsequent versions can be signed exclusively by the new private key.

## Publish an Application Release

`apps/desktop/package.json` is the only application version source. Releases use stable SemVer and
tags in the form `vX.Y.Z`.

Prepare the next version:

```powershell
cargo xtask release prepare 0.5.1
```

Review and commit the version change:

```powershell
git diff -- apps/desktop/package.json
git add apps/desktop/package.json
git commit -m "chore(release): prepare 0.5.1"
```

Create the local tag. This command requires a clean worktree and derives the tag from the package
version:

```powershell
cargo xtask release tag
```

Push the release commit and the exact tag together:

```powershell
git push origin main v0.5.1
```

The application release workflow then runs all Rust and frontend checks, builds the signed Windows
x64 NSIS installer, creates its updater signature and `latest.json`, and publishes the tag as the
GitHub latest release. The workflow fails before publishing if either signing secret is missing.

After completion, verify that the release contains the installer, its `.sig`, and `latest.json`;
that `latest.json` has a `windows-x86_64` entry; and that the GitHub latest release points to this
application version.

## Publish a Downloadable Resource

Resource tags use `resource-<id>-vX.Y.Z`. The version is read from
`resources/catalog/catalog-v1.json`; it is not passed separately to the tag command.

For a repository-owned resource, place its new payload at:

```text
resources/payloads/<id>/<version>/
```

Update the catalog descriptor with the new version, file sizes, SHA-256 values, licenses, and ordered
HTTPS URLs. Repository-owned payloads remain managed by Git LFS. Resources hosted at a fixed upstream
revision, such as the initial Hugging Face models, do not need a local payload copy.

Validate the descriptor and any local payload:

```powershell
cargo xtask resource validate lexicon-core
cargo xtask resource catalog
```

Commit the descriptor and payload changes:

```powershell
git add resources .gitattributes
git commit -m "chore(resources): release lexicon-core 1.0.1"
```

Create the local tag and push the commit with that exact tag:

```powershell
cargo xtask resource tag lexicon-core
git push origin main resource-lexicon-core-v1.0.1
```

The resource workflow selectively fetches the tagged resource's LFS payload, validates the tag,
descriptor, sizes, hashes, and licenses, publishes a frozen `resource.json`, and creates a non-latest
resource release. It updates the fixed `resource-catalog` release only after every URL in the
rendered catalog is accessible.

Multiple independently committed resource tags may be pushed together by naming every tag:

```powershell
git push origin main `
  resource-lexicon-core-v1.0.1 `
  resource-lexicon-semantic-v1.0.1
```

Catalog publication is serialized by the workflow, so concurrent resource releases cannot overwrite
the stable catalog at the same time. After completion, verify that GitHub latest still points to the
latest application release rather than a resource release.

## Local Signed NSIS Smoke Build

Use the repository-local copies without printing their contents:

```powershell
$env:GITHUB_REPOSITORY = "owner/repository"
node scripts/write-tauri-release-config.mjs apps/desktop/src-tauri/tauri.release.conf.json

$env:ATELIER_RESOURCE_CATALOG_URL = `
  "https://github.com/$env:GITHUB_REPOSITORY/releases/download/resource-catalog/catalog-v1.json"
$env:TAURI_SIGNING_PRIVATE_KEY = `
  Get-Content -Raw -LiteralPath ".secrets/tauri-updater/atelier-updater.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = `
  (Get-Content -Raw -LiteralPath ".secrets/tauri-updater/atelier-updater.password").TrimEnd()

pnpm --dir apps/desktop exec tauri build `
  --target x86_64-pc-windows-msvc `
  --bundles nsis `
  --config src-tauri/tauri.release.conf.json
```

Delete the generated `apps/desktop/src-tauri/tauri.release.conf.json` after the smoke build. CI
generates its own overlay from `GITHUB_REPOSITORY`.
