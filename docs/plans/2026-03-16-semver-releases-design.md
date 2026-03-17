# Semantic Versioned Releases — Design

## Goal

Automate versioned releases from conventional commits. Every merge to `main` accumulates changelog entries. Merging the auto-maintained Release PR tags, builds, packages, signs, and publishes artifacts for all platforms.

## Components

### 1. release-please

Google's release-please GitHub Action watches `main` for conventional commits and maintains a Release PR that bumps the workspace version in `Cargo.toml`, updates `CHANGELOG.md`, and creates a `v*` git tag on merge.

Single-version monorepo strategy — all crates share the workspace version.

### 2. PR Title Validation

`amannn/action-semantic-pull-request` validates PR titles against conventional commit format. Required status check. Since PRs are squash-merged, the PR title becomes the commit message release-please reads.

Allowed types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert.
Allowed scopes (optional): api, worker, agent, web, site, cli, linux, windows, macos, android, ios, ci, docker, deps.

### 3. Release Artifact Matrix

On `v*` tag, `release.yml` builds in parallel:

| Platform | Artifacts |
|----------|-----------|
| Linux x86_64 | static binary, .deb, .rpm, .tar.gz |
| Linux aarch64 | static binary, .deb, .rpm, .tar.gz |
| Windows x86_64 | .exe, .msi, .zip |
| macOS (universal) | universal binary, .pkg, .dmg |
| Android | .so libs (arm64-v8a, armeabi-v7a, x86_64) |
| iOS | .a static libs (aarch64, x86_64-sim, aarch64-sim) |
| Docker (api, worker, web) | Multi-arch linux/amd64 + linux/arm64, tagged version + latest |

### 4. Signing & Checksums

All native binaries and packages get Ed25519 signatures (existing tooling in `tools/signing/`). A `SHA256SUMS` file covers all artifacts. Signatures and checksums are attached to the GitHub Release.

### 5. Job Dependency Graph

```
v* tag
  -> prepare (extract version)
    -> build-* jobs (parallel: linux x2, windows, macos x2, android, ios, docker x3)
      -> package (deb, rpm, tar.gz, msi, zip, universal binary, pkg, dmg)
        -> sign-and-release (sign all, checksums, GitHub Release)
```

## Versioning Rules

- `fix:` -> patch bump
- `feat:` -> minor bump
- `feat!:` or `BREAKING CHANGE:` footer -> minor bump (pre-1.0), major bump (post-1.0)
- Pre-release detection: tags containing `-rc` or `-beta`
