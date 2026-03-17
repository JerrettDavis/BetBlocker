---
title: Contributing
description: How to contribute to BetBlocker
---


Thank you for your interest in contributing to BetBlocker. This project exists to help people regain control over gambling habits, and every contribution — whether code, documentation, or a single domain report — makes a real difference in someone's recovery.

## A Note on Context

BetBlocker serves people in gambling recovery, including individuals in vulnerable situations. Please keep this in mind when writing code, documentation, issues, and comments. Language should be respectful, supportive, and free of judgment. If you are unsure whether something is appropriate, err on the side of compassion.

## Table of Contents

- [Getting Started](#getting-started)
- [Project Structure](#project-structure)
- [How to Contribute](#how-to-contribute)
- [Code Standards](#code-standards)
- [Architecture Decision Records](#architecture-decision-records)
- [Code of Conduct](#code-of-conduct)
- [Security Issues](#security-issues)

## Getting Started

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| [Rust](https://rustup.rs/) | stable (see `rust-toolchain.toml`) | Agent, API, and worker |
| [Node.js](https://nodejs.org/) | 20+ | Web platform (Next.js) |
| [pnpm](https://pnpm.io/) | 9+ | Node package manager |
| [PostgreSQL](https://www.postgresql.org/) | 16+ | Primary database |
| [Redis](https://redis.io/) | 7+ | Cache and pub/sub |
| [Docker](https://www.docker.com/) | 24+ | Container builds and local services |
| [just](https://github.com/casey/just) | 1.0+ | Task runner (see `justfile`) |

### Development Setup

1. **Clone the repository:**

   ```bash
   git clone https://github.com/JerrettDavis/BetBlocker.git
   cd BetBlocker
   ```

2. **Install the Rust toolchain:**

   ```bash
   rustup show  # Reads rust-toolchain.toml and installs the correct version
   ```

3. **Start local services with Docker:**

   ```bash
   docker compose -f deploy/docker-compose.dev.yml up -d
   ```

   This starts PostgreSQL, Redis, and TimescaleDB for local development.

4. **Run database migrations:**

   ```bash
   just migrate
   ```

5. **Build and test the Rust workspace:**

   ```bash
   cargo build
   cargo test
   ```

6. **Set up the web platform:**

   ```bash
   cd web
   pnpm install
   pnpm dev
   ```

7. **Verify everything works:**

   ```bash
   just check  # Runs fmt, clippy, tests, and TypeScript checks
   ```

## Project Structure

BetBlocker is organized as a Cargo workspace with 15 crates and a Next.js web application.

```
betblocker/
├── crates/
│   ├── bb-common/          # Shared types, domain models, utilities
│   ├── bb-proto/           # Protocol buffer definitions and generated code
│   ├── bb-api/             # Central API server (Axum)
│   ├── bb-worker/          # Background job processor
│   ├── bb-agent-core/      # Cross-platform agent engine
│   ├── bb-agent-plugins/   # Plugin system for blocking layers
│   ├── bb-agent-linux/     # Linux agent binary
│   ├── bb-agent-windows/   # Windows agent binary
│   ├── bb-agent-macos/     # macOS agent binary
│   ├── bb-shim-windows/    # Windows platform shim (WFP, minifilter)
│   ├── bb-shim-macos/      # macOS platform shim (Network Extension)
│   ├── bb-shim-linux/      # Linux platform shim (iptables, AppArmor)
│   ├── bb-shim-android/    # Android platform shim (VpnService)
│   ├── bb-shim-ios/        # iOS platform shim (NEDNSProxyProvider)
│   └── bb-cli/             # Developer CLI tools
├── web/                    # Next.js web platform (dashboards, marketing)
├── docs/                   # Documentation and design documents
│   ├── architecture/       # Architecture decision records
│   └── plans/              # Vision and planning documents
├── deploy/                 # Docker, Helm, and deployment configs
├── migrations/             # Database migrations
├── scripts/                # Development and CI scripts
├── tests/                  # Integration and end-to-end tests
├── tools/                  # Build and development tooling
├── Cargo.toml              # Workspace root
└── justfile                # Task runner recipes
```

### Key Crates

- **bb-common**: Start here. Shared domain types (`Enrollment`, `Device`, `Blocklist`, `GamblingCategory`) used across all crates.
- **bb-api**: The central API. If you are working on enrollment flows, blocklist management, or device communication, this is where you will spend time.
- **bb-agent-core**: The cross-platform blocking engine. DNS interception, blocklist matching, and tamper detection live here.
- **bb-agent-plugins**: The plugin trait system. Blocking layers (DNS, app, browser) implement traits defined here.
- **bb-shim-***: Thin platform-specific integrations. Each shim wraps OS APIs (WFP, Network Extension, VpnService) behind the plugin interface.

## How to Contribute

### Report a Bug

Found something broken? [Open a bug report](https://github.com/JerrettDavis/BetBlocker/issues/new?template=bug_report.yml). Include your platform, the component affected, and steps to reproduce the issue.

### Request a Feature

Have an idea? [Open a feature request](https://github.com/JerrettDavis/BetBlocker/issues/new?template=feature_request.yml). Describe the use case and the problem you are trying to solve.

### Report a Gambling Domain

Know of a gambling site that BetBlocker should block? [Submit a blocklist report](https://github.com/JerrettDavis/BetBlocker/issues/new?template=blocklist_report.yml). Every domain you report helps protect someone.

### Contribute Code

1. **Fork** the repository on GitHub.
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   ```
   Use prefixes: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `chore/`.
3. **Make your changes** with tests and documentation as needed.
4. **Run the full check suite** before pushing:
   ```bash
   just check
   ```
5. **Push your branch** and open a pull request against `main`.
6. **Fill out the PR template** completely. Link any related issues.
7. **Respond to review feedback**. We aim to review PRs within 2 business days.

### Improve Documentation

Documentation improvements are always welcome. You can edit docs directly on GitHub or follow the code contribution flow above. No change is too small — fixing a typo helps.

## Code Standards

### Rust

- **Format**: Run `cargo fmt` before committing. CI enforces formatting.
- **Lint**: Run `cargo clippy`. The workspace is configured with `clippy::pedantic` — all warnings must be resolved.
- **Tests**: New functionality requires tests. Run `cargo test` to verify.
- **Unsafe code**: Denied at the workspace level. If you believe unsafe code is necessary, open an issue to discuss before implementing.
- **Error handling**: Use `thiserror` for library errors, `anyhow` for application errors. Do not use `.unwrap()` or `.expect()` in production code (enforced by clippy).

### TypeScript (web/)

- **Type checking**: Run `npx tsc --noEmit` to verify types.
- **Styling**: Tailwind CSS. Do not add custom CSS unless Tailwind cannot express the design.
- **Components**: Functional components with TypeScript interfaces for props.

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `ci`, `perf`.

Scopes match crate or directory names: `api`, `agent-core`, `worker`, `web`, `common`, `blocklist`, `deploy`.

Examples:
```
feat(agent-core): add DNS-over-HTTPS interception support
fix(api): correct enrollment tier validation on unenrollment
docs(contributing): add section on blocklist contributions
test(worker): add integration tests for delta compilation
```

### Pull Request Requirements

- All CI checks pass (formatting, linting, tests, type checking).
- No new compiler or clippy warnings.
- Tests cover new functionality.
- Description explains *what* changed and *why*.
- Documentation updated if the change affects user-facing behavior or public APIs.
- Breaking changes are clearly noted with migration instructions.

## Architecture Decision Records

Significant technical decisions are documented as Architecture Decision Records (ADRs) in `docs/architecture/`.

If your contribution involves a significant design choice — a new dependency, a changed data model, a new platform integration approach — write an ADR:

1. Copy the ADR template from `docs/architecture/template.md`.
2. Number it sequentially (e.g., `005-use-postcard-for-agent-sync.md`).
3. Fill in the context, decision, and consequences.
4. Include the ADR in your pull request.

For smaller decisions, a clear explanation in the PR description is sufficient.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to uphold these standards. Please report unacceptable behavior to the project maintainers.

## Security Issues

**Do not report security vulnerabilities through public issues.** See [SECURITY.md](SECURITY.md) for responsible disclosure instructions.

---

Thank you for helping build something that matters. Every contribution to BetBlocker is a contribution to someone's recovery.
