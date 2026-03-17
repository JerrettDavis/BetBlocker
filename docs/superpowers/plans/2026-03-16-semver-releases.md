# Semantic Versioned Releases Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Automate conventional-commit-driven semver releases with platform-specific installers, signed binaries, and multi-arch Docker images.

**Architecture:** release-please maintains a Release PR on `main` that bumps the workspace version and generates a changelog. Merging the PR creates a `v*` tag. The tag triggers `release.yml`, which builds binaries for all platforms in parallel, packages them into platform-native installers (.deb, .rpm, .msi, .pkg, .dmg, .tar.gz, .zip), builds multi-arch Docker images, signs everything with Ed25519, and publishes a GitHub Release with all artifacts. PR titles are validated against conventional commit format to ensure clean changelogs.

**Tech Stack:** release-please, GitHub Actions, cargo-deb, cargo-generate-rpm, cargo-wix, pkgbuild, create-dmg, docker buildx, Ed25519 signing

---

## File Structure

| File | Responsibility |
|------|---------------|
| `.github/workflows/release-please.yml` | NEW — release-please action, runs on push to main |
| `release-please-config.json` | NEW — release-please configuration (monorepo, single version) |
| `.release-please-manifest.json` | NEW — tracks current version for release-please |
| `.github/workflows/pr-title.yml` | NEW — conventional commit validation on PR titles |
| `.github/workflows/release.yml` | MODIFY — full rewrite with multi-platform build matrix, packaging, signing |
| `crates/bb-agent-linux/Cargo.toml` | MODIFY — add cargo-deb and cargo-generate-rpm metadata |
| `deploy/packaging/linux/betblocker-agent.service` | NEW — systemd unit file for .deb/.rpm/tar.gz |
| `deploy/packaging/linux/postinst.sh` | NEW — post-install script for deb/rpm |
| `deploy/packaging/linux/prerm.sh` | NEW — pre-remove script for deb/rpm |
| `deploy/packaging/windows/main.wxs` | NEW — WiX manifest for .msi installer |
| `deploy/packaging/macos/distribution.xml` | NEW — macOS pkg distribution descriptor |
| `deploy/packaging/macos/postinstall.sh` | NEW — macOS pkg postinstall script |
| `deploy/docker/Dockerfile.agent-linux-aarch64` | NEW — aarch64 Linux agent build |

---

## Chunk 1: release-please and PR title validation

### Task 1: Create release-please configuration

**Files:**
- Create: `release-please-config.json`
- Create: `.release-please-manifest.json`
- Create: `.github/workflows/release-please.yml`

- [ ] **Step 1: Create `release-please-config.json`**

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "release-type": "rust",
  "include-component-in-tag": false,
  "include-v-in-tag": true,
  "bump-minor-pre-major": true,
  "bump-patch-for-minor-pre-major": false,
  "changelog-sections": [
    { "type": "feat", "section": "Features" },
    { "type": "fix", "section": "Bug Fixes" },
    { "type": "perf", "section": "Performance" },
    { "type": "refactor", "section": "Code Refactoring" },
    { "type": "docs", "section": "Documentation" },
    { "type": "chore", "section": "Miscellaneous" },
    { "type": "build", "section": "Build System" },
    { "type": "ci", "section": "CI/CD" },
    { "type": "test", "section": "Tests", "hidden": true },
    { "type": "style", "section": "Styles", "hidden": true }
  ],
  "packages": {
    ".": {
      "component": "betblocker",
      "extra-files": [
        "Cargo.lock"
      ]
    }
  }
}
```

- [ ] **Step 2: Create `.release-please-manifest.json`**

This tells release-please the current version. It must match the version in `Cargo.toml` (currently `0.1.0`).

```json
{
  ".": "0.1.0"
}
```

- [ ] **Step 3: Create `.github/workflows/release-please.yml`**

```yaml
name: Release Please

on:
  push:
    branches: [main]

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    runs-on: ubuntu-latest
    steps:
      - uses: googleapis/release-please-action@v4
        with:
          config-file: release-please-config.json
          manifest-file: .release-please-manifest.json
```

- [ ] **Step 4: Verify files are valid JSON**

Run:
```bash
python -m json.tool release-please-config.json > /dev/null && echo "config OK"
python -m json.tool .release-please-manifest.json > /dev/null && echo "manifest OK"
```
Expected: Both print OK.

- [ ] **Step 5: Commit**

```bash
git add release-please-config.json .release-please-manifest.json .github/workflows/release-please.yml
git commit -m "ci: add release-please for automated semver releases"
```

---

### Task 2: Create PR title validation workflow

**Files:**
- Create: `.github/workflows/pr-title.yml`

- [ ] **Step 1: Create `.github/workflows/pr-title.yml`**

```yaml
name: PR Title

on:
  pull_request:
    types: [opened, edited, synchronize, reopened]

permissions:
  pull-requests: read

jobs:
  validate:
    name: Validate PR Title
    runs-on: ubuntu-latest
    steps:
      - uses: amannn/action-semantic-pull-request@v5
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          types: |
            feat
            fix
            docs
            style
            refactor
            perf
            test
            build
            ci
            chore
            revert
          scopes: |
            api
            worker
            agent
            web
            site
            cli
            linux
            windows
            macos
            android
            ios
            ci
            docker
            deps
          requireScope: false
          subjectPattern: ^.+$
          subjectPatternError: "PR title must not be empty after the type/scope prefix."
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/pr-title.yml
git commit -m "ci: add conventional commit validation for PR titles"
```

---

## Chunk 2: Linux packaging metadata and support files

### Task 3: Create Linux packaging support files

**Files:**
- Create: `deploy/packaging/linux/betblocker-agent.service`
- Create: `deploy/packaging/linux/betblocker-agent.conf`
- Create: `deploy/packaging/linux/postinst.sh`
- Create: `deploy/packaging/linux/prerm.sh`

- [ ] **Step 1: Create systemd unit file**

Create `deploy/packaging/linux/betblocker-agent.service`:

```ini
[Unit]
Description=BetBlocker Agent
Documentation=https://jerrettdavis.github.io/BetBlocker/
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
ExecStart=/usr/bin/betblocker-agent --config /etc/betblocker/agent.toml
Restart=on-failure
RestartSec=5
WatchdogSec=60

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/betblocker /var/log/betblocker
PrivateTmp=yes

User=betblocker
Group=betblocker

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 2: Create default config file**

Create `deploy/packaging/linux/betblocker-agent.conf`:

```toml
# BetBlocker Agent Configuration
# See https://jerrettdavis.github.io/BetBlocker/getting-started/ for details.

data_dir = "/var/lib/betblocker"

# Uncomment and set your API endpoint after enrollment:
# api_url = "https://api.betblocker.example.com"
```

- [ ] **Step 3: Create post-install script**

Create `deploy/packaging/linux/postinst.sh`:

```bash
#!/bin/sh
set -e

# Create betblocker system user if it doesn't exist
if ! getent passwd betblocker > /dev/null 2>&1; then
    useradd --system --no-create-home --shell /usr/sbin/nologin betblocker
fi

# Create data and log directories
mkdir -p /var/lib/betblocker /var/log/betblocker
chown betblocker:betblocker /var/lib/betblocker /var/log/betblocker

# Reload systemd and enable the service
systemctl daemon-reload
systemctl enable betblocker-agent.service

echo "BetBlocker agent installed. Run 'sudo systemctl start betblocker-agent' to start."
```

- [ ] **Step 4: Create pre-remove script**

Create `deploy/packaging/linux/prerm.sh`:

```bash
#!/bin/sh
set -e

# Stop and disable the service before removal
if systemctl is-active --quiet betblocker-agent.service; then
    systemctl stop betblocker-agent.service
fi
systemctl disable betblocker-agent.service 2>/dev/null || true
systemctl daemon-reload
```

- [ ] **Step 5: Add cargo-deb metadata to `crates/bb-agent-linux/Cargo.toml`**

Append the following to the end of `crates/bb-agent-linux/Cargo.toml`:

```toml
[package.metadata.deb]
maintainer = "BetBlocker Contributors <betblocker@users.noreply.github.com>"
section = "net"
priority = "optional"
depends = "systemd"
assets = [
    ["target/release/bb-agent-linux", "usr/bin/betblocker-agent", "755"],
    ["../../deploy/packaging/linux/betblocker-agent.conf", "etc/betblocker/agent.toml", "644"],
    ["../../deploy/packaging/linux/betblocker-agent.service", "lib/systemd/system/betblocker-agent.service", "644"],
]
maintainer-scripts = "../../deploy/packaging/linux/"
extended-description = "System-level agent that enforces gambling blocking policies via DNS interception, app blocking, and tamper-resistant protection."

[package.metadata.generate-rpm]
assets = [
    { source = "target/release/bb-agent-linux", dest = "/usr/bin/betblocker-agent", mode = "755" },
    { source = "../../deploy/packaging/linux/betblocker-agent.conf", dest = "/etc/betblocker/agent.toml", mode = "644", config = true },
    { source = "../../deploy/packaging/linux/betblocker-agent.service", dest = "/lib/systemd/system/betblocker-agent.service", mode = "644" },
]
post_install_script = "../../deploy/packaging/linux/postinst.sh"
pre_uninstall_script = "../../deploy/packaging/linux/prerm.sh"
```

- [ ] **Step 6: Commit**

```bash
git add deploy/packaging/linux/ crates/bb-agent-linux/Cargo.toml
git commit -m "build(linux): add systemd unit, config, and deb/rpm packaging metadata"
```

---

### Task 4: Create Windows packaging support files

**Files:**
- Create: `deploy/packaging/windows/main.wxs`

- [ ] **Step 1: Create WiX manifest**

Create `deploy/packaging/windows/main.wxs`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Package
    Name="BetBlocker Agent"
    Manufacturer="BetBlocker Contributors"
    Version="$(var.Version)"
    UpgradeCode="8B2F4E3A-1C5D-4A8E-B9F7-6D3E2A1C5B4D">

    <MajorUpgrade
      DowngradeErrorMessage="A newer version of BetBlocker Agent is already installed." />

    <MediaTemplate EmbedCab="yes" />

    <Feature Id="MainFeature" Title="BetBlocker Agent" Level="1">
      <ComponentGroupRef Id="AgentBinary" />
      <ComponentGroupRef Id="AgentConfig" />
      <ComponentRef Id="ServiceInstaller" />
    </Feature>

    <StandardDirectory Id="ProgramFiles64Folder">
      <Directory Id="INSTALLFOLDER" Name="BetBlocker">
        <Component Id="ServiceInstaller" Guid="A1B2C3D4-E5F6-7890-ABCD-EF1234567890">
          <ServiceInstall
            Id="BetBlockerAgentService"
            Name="BetBlockerAgent"
            DisplayName="BetBlocker Agent"
            Description="System-level gambling blocking agent"
            Type="ownProcess"
            Start="auto"
            ErrorControl="normal"
            Account="LocalSystem" />
          <ServiceControl
            Id="StartBetBlockerAgent"
            Name="BetBlockerAgent"
            Start="install"
            Stop="both"
            Remove="uninstall"
            Wait="yes" />
        </Component>
      </Directory>
    </StandardDirectory>

    <ComponentGroup Id="AgentBinary" Directory="INSTALLFOLDER">
      <Component>
        <File Source="$(var.BinaryPath)" Name="betblocker-agent.exe" KeyPath="yes" />
      </Component>
    </ComponentGroup>

    <ComponentGroup Id="AgentConfig" Directory="INSTALLFOLDER">
      <Component>
        <File Source="$(var.ConfigPath)" Name="agent.toml" KeyPath="yes" />
      </Component>
    </ComponentGroup>
  </Package>
</Wix>
```

- [ ] **Step 2: Create Windows default config**

Create `deploy/packaging/windows/betblocker-agent.conf`:

```toml
# BetBlocker Agent Configuration
# See https://jerrettdavis.github.io/BetBlocker/platform-guides/windows/ for details.

data_dir = "C:\\ProgramData\\BetBlocker"

# Uncomment and set your API endpoint after enrollment:
# api_url = "https://api.betblocker.example.com"
```

- [ ] **Step 3: Commit**

```bash
git add deploy/packaging/windows/
git commit -m "build(windows): add WiX manifest and default config for MSI installer"
```

---

### Task 5: Create macOS packaging support files

**Files:**
- Create: `deploy/packaging/macos/distribution.xml`
- Create: `deploy/packaging/macos/postinstall.sh`
- Create: `deploy/packaging/macos/com.betblocker.agent.plist`

- [ ] **Step 1: Create LaunchDaemon plist**

Create `deploy/packaging/macos/com.betblocker.agent.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.betblocker.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Library/BetBlocker/betblocker-agent</string>
        <string>--config</string>
        <string>/Library/Application Support/BetBlocker/agent.toml</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/Library/Logs/BetBlocker/agent.log</string>
    <key>StandardErrorPath</key>
    <string>/Library/Logs/BetBlocker/agent.err</string>
</dict>
</plist>
```

- [ ] **Step 2: Create postinstall script**

Create `deploy/packaging/macos/postinstall.sh`:

```bash
#!/bin/bash
set -e

# Create directories
mkdir -p "/Library/Application Support/BetBlocker"
mkdir -p "/Library/Logs/BetBlocker"
mkdir -p "/Library/BetBlocker"

# Copy LaunchDaemon plist
cp "/Library/BetBlocker/com.betblocker.agent.plist" "/Library/LaunchDaemons/"

# Install default config if none exists
if [ ! -f "/Library/Application Support/BetBlocker/agent.toml" ]; then
    cp "/Library/BetBlocker/agent.toml" "/Library/Application Support/BetBlocker/agent.toml"
fi

# Load the daemon
launchctl bootstrap system "/Library/LaunchDaemons/com.betblocker.agent.plist" 2>/dev/null || true

echo "BetBlocker agent installed."
```

- [ ] **Step 3: Create macOS default config**

Create `deploy/packaging/macos/betblocker-agent.conf`:

```toml
# BetBlocker Agent Configuration
# See https://jerrettdavis.github.io/BetBlocker/platform-guides/macos/ for details.

data_dir = "/Library/Application Support/BetBlocker"

# Uncomment and set your API endpoint after enrollment:
# api_url = "https://api.betblocker.example.com"
```

- [ ] **Step 4: Create distribution.xml**

Create `deploy/packaging/macos/distribution.xml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<installer-gui-script minSpecVersion="2">
    <title>BetBlocker Agent</title>
    <organization>com.betblocker</organization>
    <domains enable_localSystem="true" />
    <options customize="never" require-scripts="true" rootVolumeOnly="true" />
    <allowed-os-versions>
        <os-version min="11.0" />
    </allowed-os-versions>
    <choices-outline>
        <line choice="default">
            <line choice="com.betblocker.agent.pkg" />
        </line>
    </choices-outline>
    <choice id="default" />
    <choice id="com.betblocker.agent.pkg" visible="false">
        <pkg-ref id="com.betblocker.agent.pkg" />
    </choice>
    <pkg-ref id="com.betblocker.agent.pkg"
             version="0.0.0"
             onConclusion="none">agent.pkg</pkg-ref>
</installer-gui-script>
```

- [ ] **Step 5: Commit**

```bash
git add deploy/packaging/macos/
git commit -m "build(macos): add LaunchDaemon plist, pkg distribution, and postinstall script"
```

---

## Chunk 3: Release workflow (full rewrite)

### Task 6: Rewrite release.yml with full platform matrix

**Files:**
- Modify: `.github/workflows/release.yml`
- Create: `deploy/docker/Dockerfile.agent-linux-aarch64`

- [ ] **Step 1: Create aarch64 Linux agent Dockerfile**

Create `deploy/docker/Dockerfile.agent-linux-aarch64`:

```dockerfile
# deploy/docker/Dockerfile.agent-linux-aarch64
# Purpose: Build the Linux aarch64 agent binary in a reproducible environment.
# ---------------------------------------------------------
# Stage 1: Build static agent binary for aarch64
# ---------------------------------------------------------
FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev protobuf-dev gcc-aarch64-none-elf

RUN rustup target add aarch64-unknown-linux-musl

WORKDIR /build

# Copy workspace manifests for dependency caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY .cargo .cargo

# Override cross-compilation linker for Alpine
RUN sed -i '/^\[target\.aarch64-unknown-linux-musl\]/,/^$/s/^linker.*/# linker auto-detected on Alpine/' .cargo/config.toml
COPY crates/bb-common/Cargo.toml crates/bb-common/Cargo.toml
COPY crates/bb-proto/Cargo.toml crates/bb-proto/Cargo.toml
COPY crates/bb-api/Cargo.toml crates/bb-api/Cargo.toml
COPY crates/bb-worker/Cargo.toml crates/bb-worker/Cargo.toml
COPY crates/bb-agent-core/Cargo.toml crates/bb-agent-core/Cargo.toml
COPY crates/bb-agent-plugins/Cargo.toml crates/bb-agent-plugins/Cargo.toml
COPY crates/bb-agent-linux/Cargo.toml crates/bb-agent-linux/Cargo.toml
COPY crates/bb-agent-windows/Cargo.toml crates/bb-agent-windows/Cargo.toml
COPY crates/bb-agent-macos/Cargo.toml crates/bb-agent-macos/Cargo.toml
COPY crates/bb-shim-windows/Cargo.toml crates/bb-shim-windows/Cargo.toml
COPY crates/bb-shim-macos/Cargo.toml crates/bb-shim-macos/Cargo.toml
COPY crates/bb-shim-linux/Cargo.toml crates/bb-shim-linux/Cargo.toml
COPY crates/bb-shim-android/Cargo.toml crates/bb-shim-android/Cargo.toml
COPY crates/bb-shim-ios/Cargo.toml crates/bb-shim-ios/Cargo.toml
COPY crates/bb-cli/Cargo.toml crates/bb-cli/Cargo.toml

# Create dummy source files for dependency caching
RUN mkdir -p crates/bb-common/src && echo "" > crates/bb-common/src/lib.rs && \
    mkdir -p crates/bb-proto/src && echo "" > crates/bb-proto/src/lib.rs && \
    mkdir -p crates/bb-proto/proto && touch crates/bb-proto/proto/device.proto && \
    mkdir -p crates/bb-api/src && echo "fn main() {}" > crates/bb-api/src/main.rs && \
    mkdir -p crates/bb-worker/src && echo "fn main() {}" > crates/bb-worker/src/main.rs && \
    mkdir -p crates/bb-agent-core/src && echo "" > crates/bb-agent-core/src/lib.rs && \
    mkdir -p crates/bb-agent-plugins/src && echo "" > crates/bb-agent-plugins/src/lib.rs && \
    mkdir -p crates/bb-agent-linux/src && echo "fn main() {}" > crates/bb-agent-linux/src/main.rs && \
    mkdir -p crates/bb-agent-windows/src && echo "fn main() {}" > crates/bb-agent-windows/src/main.rs && \
    mkdir -p crates/bb-agent-macos/src && echo "fn main() {}" > crates/bb-agent-macos/src/main.rs && \
    mkdir -p crates/bb-shim-windows/src && echo "" > crates/bb-shim-windows/src/lib.rs && \
    mkdir -p crates/bb-shim-macos/src && echo "" > crates/bb-shim-macos/src/lib.rs && \
    mkdir -p crates/bb-shim-linux/src && echo "" > crates/bb-shim-linux/src/lib.rs && \
    mkdir -p crates/bb-shim-android/src && echo "" > crates/bb-shim-android/src/lib.rs && \
    mkdir -p crates/bb-shim-ios/src && echo "" > crates/bb-shim-ios/src/lib.rs && \
    mkdir -p crates/bb-cli/src && echo "fn main() {}" > crates/bb-cli/src/main.rs

RUN cargo build --release --target aarch64-unknown-linux-musl -p bb-agent-linux 2>/dev/null || true

COPY crates crates

RUN find crates -name "*.rs" -exec touch {} +

RUN cargo build --release --target aarch64-unknown-linux-musl -p bb-agent-linux

# ---------------------------------------------------------
# Stage 2: Extract binary
# ---------------------------------------------------------
FROM scratch

COPY --from=builder /build/target/aarch64-unknown-linux-musl/release/bb-agent-linux /bb-agent-linux
```

- [ ] **Step 2: Rewrite `.github/workflows/release.yml`**

Replace the entire contents of `.github/workflows/release.yml` with:

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags: ["v*"]

concurrency:
  group: release-${{ github.ref_name }}
  cancel-in-progress: false

env:
  REGISTRY: ghcr.io

permissions:
  contents: write
  packages: write

jobs:
  # -----------------------------------------------------------
  # Extract version from tag
  # -----------------------------------------------------------
  prepare:
    runs-on: ubuntu-latest
    outputs:
      version: ${{ steps.version.outputs.version }}
    steps:
      - name: Extract version
        id: version
        run: echo "version=${GITHUB_REF_NAME#v}" >> "$GITHUB_OUTPUT"

  # -----------------------------------------------------------
  # Multi-arch Docker images (api, worker, web)
  # -----------------------------------------------------------
  build-api-image:
    needs: prepare
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Compute lowercase image prefix
        run: echo "IMAGE_PREFIX=ghcr.io/${GITHUB_REPOSITORY_OWNER,,}/betblocker" >> "$GITHUB_ENV"

      - uses: docker/setup-qemu-action@v3
      - uses: docker/setup-buildx-action@v3

      - uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push API image (multi-arch)
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.api
          push: true
          platforms: linux/amd64,linux/arm64
          tags: |
            ${{ env.IMAGE_PREFIX }}-api:${{ needs.prepare.outputs.version }}
            ${{ env.IMAGE_PREFIX }}-api:latest
          cache-from: type=gha,scope=api-release
          cache-to: type=gha,mode=max,scope=api-release

  build-worker-image:
    needs: prepare
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Compute lowercase image prefix
        run: echo "IMAGE_PREFIX=ghcr.io/${GITHUB_REPOSITORY_OWNER,,}/betblocker" >> "$GITHUB_ENV"

      - uses: docker/setup-qemu-action@v3
      - uses: docker/setup-buildx-action@v3

      - uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push worker image (multi-arch)
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.worker
          push: true
          platforms: linux/amd64,linux/arm64
          tags: |
            ${{ env.IMAGE_PREFIX }}-worker:${{ needs.prepare.outputs.version }}
            ${{ env.IMAGE_PREFIX }}-worker:latest
          cache-from: type=gha,scope=worker-release
          cache-to: type=gha,mode=max,scope=worker-release

  build-web-image:
    needs: prepare
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Compute lowercase image prefix
        run: echo "IMAGE_PREFIX=ghcr.io/${GITHUB_REPOSITORY_OWNER,,}/betblocker" >> "$GITHUB_ENV"

      - uses: docker/setup-qemu-action@v3
      - uses: docker/setup-buildx-action@v3

      - uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push web image (multi-arch)
        uses: docker/build-push-action@v6
        with:
          context: ./web
          file: deploy/docker/Dockerfile.web
          push: true
          platforms: linux/amd64,linux/arm64
          tags: |
            ${{ env.IMAGE_PREFIX }}-web:${{ needs.prepare.outputs.version }}
            ${{ env.IMAGE_PREFIX }}-web:latest
          cache-from: type=gha,scope=web-release
          cache-to: type=gha,mode=max,scope=web-release

  # -----------------------------------------------------------
  # Linux agent binaries (x86_64 + aarch64)
  # -----------------------------------------------------------
  build-linux-x86_64:
    needs: prepare
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: docker/setup-buildx-action@v3

      - name: Build agent binary via Docker
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.agent-linux
          push: false
          load: true
          tags: bb-agent-linux-x86_64:build
          cache-from: type=gha,scope=agent-linux-x86_64
          cache-to: type=gha,mode=max,scope=agent-linux-x86_64

      - name: Extract binary
        run: |
          docker create --name extract bb-agent-linux-x86_64:build /bb-agent-linux
          docker cp extract:/bb-agent-linux ./betblocker-agent-linux-x86_64
          docker rm extract
          chmod +x betblocker-agent-linux-x86_64

      - uses: actions/upload-artifact@v4
        with:
          name: betblocker-agent-linux-x86_64
          path: betblocker-agent-linux-x86_64

  build-linux-aarch64:
    needs: prepare
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: docker/setup-buildx-action@v3

      - name: Build agent binary via Docker
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.agent-linux-aarch64
          push: false
          load: true
          tags: bb-agent-linux-aarch64:build
          cache-from: type=gha,scope=agent-linux-aarch64
          cache-to: type=gha,mode=max,scope=agent-linux-aarch64

      - name: Extract binary
        run: |
          docker create --name extract bb-agent-linux-aarch64:build /bb-agent-linux
          docker cp extract:/bb-agent-linux ./betblocker-agent-linux-aarch64
          docker rm extract
          chmod +x betblocker-agent-linux-aarch64

      - uses: actions/upload-artifact@v4
        with:
          name: betblocker-agent-linux-aarch64
          path: betblocker-agent-linux-aarch64

  # -----------------------------------------------------------
  # Windows agent binary
  # -----------------------------------------------------------
  build-windows:
    needs: prepare
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-pc-windows-msvc

      - uses: Swatinem/rust-cache@v2

      - name: Install protoc
        uses: arduino/setup-protoc@v3
        with:
          repo-token: ${{ secrets.GITHUB_TOKEN }}

      - name: Build Windows agent
        run: cargo build --release --target x86_64-pc-windows-msvc -p bb-agent-windows

      - name: Copy binary
        run: |
          Copy-Item "target\x86_64-pc-windows-msvc\release\bb-agent-windows.exe" "betblocker-agent-windows-x86_64.exe"

      - uses: actions/upload-artifact@v4
        with:
          name: betblocker-agent-windows-x86_64
          path: betblocker-agent-windows-x86_64.exe

  # -----------------------------------------------------------
  # macOS agent binaries (x86_64 + aarch64)
  # -----------------------------------------------------------
  build-macos-x86_64:
    needs: prepare
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-apple-darwin

      - uses: Swatinem/rust-cache@v2

      - name: Install protoc
        run: brew install protobuf

      - name: Build macOS agent (x86_64)
        run: cargo build --release --target x86_64-apple-darwin -p bb-agent-macos

      - name: Copy binary
        run: cp target/x86_64-apple-darwin/release/bb-agent-macos betblocker-agent-macos-x86_64

      - uses: actions/upload-artifact@v4
        with:
          name: betblocker-agent-macos-x86_64
          path: betblocker-agent-macos-x86_64

  build-macos-aarch64:
    needs: prepare
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin

      - uses: Swatinem/rust-cache@v2

      - name: Install protoc
        run: brew install protobuf

      - name: Build macOS agent (aarch64)
        run: cargo build --release --target aarch64-apple-darwin -p bb-agent-macos

      - name: Copy binary
        run: cp target/aarch64-apple-darwin/release/bb-agent-macos betblocker-agent-macos-aarch64

      - uses: actions/upload-artifact@v4
        with:
          name: betblocker-agent-macos-aarch64
          path: betblocker-agent-macos-aarch64

  # -----------------------------------------------------------
  # Android shared libraries
  # -----------------------------------------------------------
  build-android:
    needs: prepare
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android,armv7-linux-androideabi,x86_64-linux-android

      - uses: Swatinem/rust-cache@v2

      - name: Install protoc
        run: sudo apt-get install -y protobuf-compiler

      - name: Install cargo-ndk
        run: cargo install cargo-ndk

      - name: Set up Android NDK
        uses: android-actions/setup-android@v3

      - name: Build Android shim libraries
        run: |
          cargo ndk \
            -t arm64-v8a \
            -t armeabi-v7a \
            -t x86_64 \
            -o android-libs \
            build --release -p bb-shim-android

      - name: Package Android libraries
        run: tar czf betblocker-android-libs.tar.gz -C android-libs .

      - uses: actions/upload-artifact@v4
        with:
          name: betblocker-android-libs
          path: betblocker-android-libs.tar.gz

  # -----------------------------------------------------------
  # iOS static libraries
  # -----------------------------------------------------------
  build-ios:
    needs: prepare
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-ios,aarch64-apple-ios-sim,x86_64-apple-ios

      - uses: Swatinem/rust-cache@v2

      - name: Install protoc
        run: brew install protobuf

      - name: Build iOS shim libraries
        run: |
          cargo build --release --target aarch64-apple-ios -p bb-shim-ios
          cargo build --release --target aarch64-apple-ios-sim -p bb-shim-ios
          cargo build --release --target x86_64-apple-ios -p bb-shim-ios

      - name: Package iOS libraries
        run: |
          mkdir -p ios-libs/device ios-libs/simulator
          cp target/aarch64-apple-ios/release/libbb_shim_ios.a ios-libs/device/
          # Create simulator fat library
          lipo -create \
            target/aarch64-apple-ios-sim/release/libbb_shim_ios.a \
            target/x86_64-apple-ios/release/libbb_shim_ios.a \
            -output ios-libs/simulator/libbb_shim_ios.a
          tar czf betblocker-ios-libs.tar.gz -C ios-libs .

      - uses: actions/upload-artifact@v4
        with:
          name: betblocker-ios-libs
          path: betblocker-ios-libs.tar.gz

  # -----------------------------------------------------------
  # Package: Linux .deb, .rpm, .tar.gz
  # -----------------------------------------------------------
  package-linux:
    needs: [prepare, build-linux-x86_64, build-linux-aarch64]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Install protoc
        run: sudo apt-get install -y protobuf-compiler

      - name: Install packaging tools
        run: |
          cargo install cargo-deb cargo-generate-rpm

      - name: Download Linux binaries
        uses: actions/download-artifact@v4
        with:
          pattern: betblocker-agent-linux-*
          merge-multiple: false

      - name: Build .deb (x86_64)
        run: |
          mkdir -p target/release
          cp betblocker-agent-linux-x86_64/betblocker-agent-linux-x86_64 target/release/bb-agent-linux
          chmod +x target/release/bb-agent-linux
          cargo deb -p bb-agent-linux --no-build --no-strip
          cp target/debian/*.deb betblocker-agent_${{ needs.prepare.outputs.version }}_amd64.deb

      - name: Build .rpm (x86_64)
        run: |
          cargo generate-rpm -p crates/bb-agent-linux
          cp target/generate-rpm/*.rpm betblocker-agent-${{ needs.prepare.outputs.version }}.x86_64.rpm

      - name: Build .tar.gz (x86_64)
        run: |
          mkdir -p betblocker-agent-${{ needs.prepare.outputs.version }}-linux-x86_64
          cp betblocker-agent-linux-x86_64/betblocker-agent-linux-x86_64 betblocker-agent-${{ needs.prepare.outputs.version }}-linux-x86_64/betblocker-agent
          cp deploy/packaging/linux/betblocker-agent.service betblocker-agent-${{ needs.prepare.outputs.version }}-linux-x86_64/
          cp deploy/packaging/linux/betblocker-agent.conf betblocker-agent-${{ needs.prepare.outputs.version }}-linux-x86_64/agent.toml
          tar czf betblocker-agent-${{ needs.prepare.outputs.version }}-linux-x86_64.tar.gz \
            betblocker-agent-${{ needs.prepare.outputs.version }}-linux-x86_64/

      - name: Build .tar.gz (aarch64)
        run: |
          mkdir -p betblocker-agent-${{ needs.prepare.outputs.version }}-linux-aarch64
          cp betblocker-agent-linux-aarch64/betblocker-agent-linux-aarch64 betblocker-agent-${{ needs.prepare.outputs.version }}-linux-aarch64/betblocker-agent
          cp deploy/packaging/linux/betblocker-agent.service betblocker-agent-${{ needs.prepare.outputs.version }}-linux-aarch64/
          cp deploy/packaging/linux/betblocker-agent.conf betblocker-agent-${{ needs.prepare.outputs.version }}-linux-aarch64/agent.toml
          tar czf betblocker-agent-${{ needs.prepare.outputs.version }}-linux-aarch64.tar.gz \
            betblocker-agent-${{ needs.prepare.outputs.version }}-linux-aarch64/

      - uses: actions/upload-artifact@v4
        with:
          name: linux-packages
          path: |
            betblocker-agent_*.deb
            betblocker-agent-*.rpm
            betblocker-agent-*-linux-*.tar.gz

  # -----------------------------------------------------------
  # Package: Windows .msi, .zip
  # -----------------------------------------------------------
  package-windows:
    needs: [prepare, build-windows]
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download Windows binary
        uses: actions/download-artifact@v4
        with:
          name: betblocker-agent-windows-x86_64

      - name: Install WiX Toolset
        run: dotnet tool install --global wix

      - name: Build MSI
        run: |
          wix build deploy\packaging\windows\main.wxs `
            -d Version=${{ needs.prepare.outputs.version }} `
            -d BinaryPath=${{ github.workspace }}\betblocker-agent-windows-x86_64.exe `
            -d ConfigPath=${{ github.workspace }}\deploy\packaging\windows\betblocker-agent.conf `
            -arch x64 `
            -o betblocker-agent-${{ needs.prepare.outputs.version }}-windows-x86_64.msi

      - name: Build ZIP
        run: |
          $dir = "betblocker-agent-${{ needs.prepare.outputs.version }}-windows-x86_64"
          New-Item -ItemType Directory -Path $dir
          Copy-Item betblocker-agent-windows-x86_64.exe "$dir\betblocker-agent.exe"
          Copy-Item deploy\packaging\windows\betblocker-agent.conf "$dir\agent.toml"
          Compress-Archive -Path $dir -DestinationPath "$dir.zip"

      - uses: actions/upload-artifact@v4
        with:
          name: windows-packages
          path: |
            betblocker-agent-*-windows-*.msi
            betblocker-agent-*-windows-*.zip

  # -----------------------------------------------------------
  # Package: macOS universal binary, .pkg, .dmg
  # -----------------------------------------------------------
  package-macos:
    needs: [prepare, build-macos-x86_64, build-macos-aarch64]
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download macOS binaries
        uses: actions/download-artifact@v4
        with:
          pattern: betblocker-agent-macos-*
          merge-multiple: false

      - name: Create universal binary
        run: |
          chmod +x betblocker-agent-macos-x86_64/betblocker-agent-macos-x86_64
          chmod +x betblocker-agent-macos-aarch64/betblocker-agent-macos-aarch64
          lipo -create \
            betblocker-agent-macos-x86_64/betblocker-agent-macos-x86_64 \
            betblocker-agent-macos-aarch64/betblocker-agent-macos-aarch64 \
            -output betblocker-agent-macos-universal

      - name: Build .pkg
        run: |
          VERSION="${{ needs.prepare.outputs.version }}"

          # Create package root
          mkdir -p pkg-root/Library/BetBlocker
          cp betblocker-agent-macos-universal pkg-root/Library/BetBlocker/betblocker-agent
          chmod +x pkg-root/Library/BetBlocker/betblocker-agent
          cp deploy/packaging/macos/com.betblocker.agent.plist pkg-root/Library/BetBlocker/
          cp deploy/packaging/macos/betblocker-agent.conf pkg-root/Library/BetBlocker/agent.toml

          # Create scripts directory
          mkdir -p scripts
          cp deploy/packaging/macos/postinstall.sh scripts/postinstall
          chmod +x scripts/postinstall

          # Build component package
          pkgbuild \
            --root pkg-root \
            --scripts scripts \
            --identifier com.betblocker.agent \
            --version "${VERSION}" \
            --install-location / \
            agent.pkg

          # Build distribution package
          sed "s/0.0.0/${VERSION}/" deploy/packaging/macos/distribution.xml > distribution.xml
          productbuild \
            --distribution distribution.xml \
            --package-path . \
            "betblocker-agent-${VERSION}-macos-universal.pkg"

      - name: Build .dmg
        run: |
          VERSION="${{ needs.prepare.outputs.version }}"
          DMG_NAME="betblocker-agent-${VERSION}-macos-universal"

          mkdir -p dmg-contents
          cp "betblocker-agent-${VERSION}-macos-universal.pkg" dmg-contents/
          hdiutil create \
            -volname "BetBlocker Agent ${VERSION}" \
            -srcfolder dmg-contents \
            -ov \
            -format UDZO \
            "${DMG_NAME}.dmg"

      - uses: actions/upload-artifact@v4
        with:
          name: macos-packages
          path: |
            betblocker-agent-*-macos-universal
            betblocker-agent-*-macos-universal.pkg
            betblocker-agent-*-macos-universal.dmg

  # -----------------------------------------------------------
  # Sign all artifacts and create GitHub Release
  # -----------------------------------------------------------
  sign-and-release:
    needs:
      - prepare
      - build-api-image
      - build-worker-image
      - build-web-image
      - package-linux
      - package-windows
      - package-macos
      - build-android
      - build-ios
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Flatten artifacts
        run: |
          mkdir -p release-files
          find artifacts -type f \( \
            -name "*.deb" -o -name "*.rpm" -o -name "*.tar.gz" \
            -o -name "*.msi" -o -name "*.zip" \
            -o -name "*.pkg" -o -name "*.dmg" \
            -o -name "betblocker-agent-*" \
          \) -exec cp {} release-files/ \;

      - name: Sign all binaries and packages
        env:
          SIGNING_KEY_BASE64: ${{ secrets.ED25519_RELEASE_SIGNING_KEY }}
        run: |
          if [ -z "${SIGNING_KEY_BASE64}" ]; then
            echo "::warning::ED25519_RELEASE_SIGNING_KEY not set — skipping signing"
            exit 0
          fi

          echo "${SIGNING_KEY_BASE64}" | base64 -d > /tmp/release-signing.key

          for file in release-files/*; do
            [ -f "${file}" ] || continue
            echo "Signing $(basename "${file}")..."
            bash tools/signing/sign-binary.sh \
              "${file}" \
              /tmp/release-signing.key \
              "${file}.sig"
          done

          rm -f /tmp/release-signing.key

      - name: Generate SHA256SUMS
        run: |
          cd release-files
          sha256sum * > SHA256SUMS
          cd ..

      - name: Compute lowercase image prefix
        run: echo "IMAGE_PREFIX=ghcr.io/${GITHUB_REPOSITORY_OWNER,,}/betblocker" >> "$GITHUB_ENV"

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ github.ref_name }}
          name: BetBlocker ${{ github.ref_name }}
          draft: false
          prerelease: ${{ contains(github.ref_name, '-rc') || contains(github.ref_name, '-beta') }}
          generate_release_notes: false
          body: |
            ## Docker Images

            ```bash
            docker pull ${{ env.IMAGE_PREFIX }}-api:${{ needs.prepare.outputs.version }}
            docker pull ${{ env.IMAGE_PREFIX }}-worker:${{ needs.prepare.outputs.version }}
            docker pull ${{ env.IMAGE_PREFIX }}-web:${{ needs.prepare.outputs.version }}
            ```

            All images support `linux/amd64` and `linux/arm64`.

            ## Self-Hosted Upgrade

            ```bash
            cd deploy
            echo "BETBLOCKER_VERSION=${{ needs.prepare.outputs.version }}" >> .env
            docker compose pull && docker compose up -d
            ```

            ## Agent Downloads

            | Platform | Packages |
            |----------|----------|
            | Linux x86_64 | `.deb` `.rpm` `.tar.gz` |
            | Linux aarch64 | `.tar.gz` |
            | Windows x86_64 | `.msi` `.zip` |
            | macOS Universal | `.pkg` `.dmg` |
            | Android | `.tar.gz` (shared libs) |
            | iOS | `.tar.gz` (static libs) |

            All binaries have `.sig` Ed25519 signatures. Verify with:
            ```bash
            bash tools/signing/verify-binary.sh <file> <file>.sig
            ```

            See `SHA256SUMS` for checksums.
          files: release-files/*
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml deploy/docker/Dockerfile.agent-linux-aarch64
git commit -m "ci: rewrite release workflow with full platform matrix, packaging, and multi-arch Docker"
```

---

## Chunk 4: Verification

### Task 7: Validate all workflow files and commit everything

- [ ] **Step 1: Validate YAML syntax of all workflows**

Run:
```bash
python -c "
import yaml, sys, pathlib
for f in pathlib.Path('.github/workflows').glob('*.yml'):
    try:
        yaml.safe_load(f.read_text())
        print(f'OK: {f}')
    except Exception as e:
        print(f'FAIL: {f}: {e}')
        sys.exit(1)
"
```
Expected: All files print OK.

- [ ] **Step 2: Verify all packaging support files exist**

Run:
```bash
ls -la deploy/packaging/linux/betblocker-agent.service
ls -la deploy/packaging/linux/betblocker-agent.conf
ls -la deploy/packaging/linux/postinst.sh
ls -la deploy/packaging/linux/prerm.sh
ls -la deploy/packaging/windows/main.wxs
ls -la deploy/packaging/windows/betblocker-agent.conf
ls -la deploy/packaging/macos/distribution.xml
ls -la deploy/packaging/macos/postinstall.sh
ls -la deploy/packaging/macos/com.betblocker.agent.plist
ls -la deploy/packaging/macos/betblocker-agent.conf
ls -la deploy/docker/Dockerfile.agent-linux-aarch64
```
Expected: All files exist.

- [ ] **Step 3: Verify release-please config**

Run:
```bash
python -m json.tool release-please-config.json > /dev/null && echo "config OK"
python -m json.tool .release-please-manifest.json > /dev/null && echo "manifest OK"
```
Expected: Both OK.

- [ ] **Step 4: Final commit with all files if any were missed**

```bash
git status
# If any unstaged files, add and commit them
```
