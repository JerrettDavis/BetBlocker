# ADR-001: Rust for Endpoint Agent Core

## Status
Proposed

## Date
2026-03-12

## Context

BetBlocker's endpoint agent is a system-level service that runs on five platforms (Windows, macOS, Linux, Android, iOS) with the following hard requirements:

1. **Memory safety without garbage collection.** The agent intercepts DNS queries, inspects network traffic, and manages cryptographic material. A use-after-free or buffer overflow in a privileged process is a security vulnerability that undermines the product's entire value proposition. A garbage collector introduces unpredictable latency in the DNS resolution hot path and complicates real-time tamper detection.

2. **Cross-platform compilation from a single codebase.** Maintaining five separate native codebases (one per OS) is economically infeasible for a small team. The core blocking engine, blocklist matching, event reporting, and API communication must be written once and compiled per target.

3. **Native performance for DNS interception.** The agent sits in the critical path of every DNS query on the device. Latency must be imperceptible (sub-millisecond for blocklist lookup). The binary must be small enough for mobile distribution (target: under 10 MB stripped).

4. **FFI compatibility with OS-level APIs.** Platform shims need to call into WFP (Windows), Network Extension (macOS/iOS), VpnService (Android), and iptables/nftables (Linux). The core language must have zero-cost FFI to C, Swift, Kotlin/JNI, and Objective-C.

5. **Tamper resistance properties.** The compiled binary should be difficult to patch, and the language should support compile-time guarantees that reduce the attack surface (no null pointer dereferences, no data races).

6. **Shared type system with the API.** The central API (Axum) and background worker also need to be high-performance Rust. Sharing domain types (blocklist entries, enrollment structures, event schemas) between agent and API eliminates serialization mismatches and reduces bugs at the network boundary.

## Decision

**Use Rust as the sole language for the endpoint agent core, the central API, and the background worker.**

The agent codebase will be structured as a Cargo workspace:

```
betblocker-agent/
  crates/
    bb-core/          # Cross-platform blocking engine, blocklist, event system
    bb-dns/           # DNS resolver and interception
    bb-net/           # API client, mTLS, certificate pinning
    bb-tamper/        # Integrity checks, watchdog protocol, hardware key access
    bb-plugin/        # Plugin trait definitions and loader
    bb-agent/         # Binary entrypoint, service lifecycle, orchestration
    bb-shim-windows/  # WFP, minifilter FFI
    bb-shim-macos/    # Network Extension, System Extension FFI
    bb-shim-linux/    # iptables/nftables, AppArmor integration
    bb-shim-android/  # VpnService JNI bridge, Device Admin
    bb-shim-ios/      # NEDNSProxyProvider, MDM profile integration
```

Platform shims are thin Rust crates that use `#[cfg(target_os = "...")]` for conditional compilation. Where Rust FFI alone is insufficient (e.g., Swift-only APIs on macOS, Kotlin APIs on Android), the shim crate calls into a minimal native library via C-ABI bindings. These native libraries are kept under 500 lines each and exist solely as bridges.

### Compilation Targets

| Platform | Target Triple | Toolchain Notes |
|----------|---------------|-----------------|
| Windows x86_64 | `x86_64-pc-windows-msvc` | MSVC linker, links to WFP/kernel headers |
| Windows ARM64 | `aarch64-pc-windows-msvc` | Same, ARM64 builds |
| macOS x86_64 | `x86_64-apple-darwin` | Xcode toolchain for framework linking |
| macOS ARM64 | `aarch64-apple-darwin` | Universal binary via `lipo` |
| Linux x86_64 | `x86_64-unknown-linux-musl` | Static linking for distro independence |
| Linux ARM64 | `aarch64-unknown-linux-musl` | Same, ARM64 |
| Android ARM64 | `aarch64-linux-android` | NDK toolchain, JNI for service bridge |
| Android x86_64 | `x86_64-linux-android` | Emulator support |
| iOS ARM64 | `aarch64-apple-ios` | Xcode toolchain, limited to extension sandbox |

### Key Crate Dependencies (Initial)

- `hickory-dns` (formerly trust-dns) for DNS resolution
- `rustls` for TLS (no OpenSSL dependency, pure Rust)
- `ring` for cryptographic primitives
- `serde` / `serde_json` / `postcard` for serialization
- `tokio` for async runtime (single-threaded runtime on mobile to minimize footprint)
- `axum` for the API server (shared types with agent via `bb-core`)

## Alternatives Considered

### Go

**Pros:** Fast compilation, built-in concurrency, large ecosystem, simpler learning curve, good cross-compilation story.

**Rejected because:**
- Garbage collector introduces unpredictable pauses. For a DNS interceptor in the hot path, this is unacceptable. Go's GC latency is typically under 1ms, but under memory pressure it can spike, and on resource-constrained mobile devices it becomes a real concern.
- Binary size is significantly larger (30-50 MB typical for a Go binary vs 5-10 MB for Rust). On mobile platforms this matters for app store approval and user willingness to install.
- CGo FFI has substantial overhead and complexity. Calling into C libraries from Go requires CGo, which disables cross-compilation, adds build complexity, and has non-trivial per-call overhead. The agent makes thousands of FFI calls per second through platform shims.
- No shared type system with a Rust API. If the API were also Go this would be neutral, but Go's type system lacks the expressiveness for the domain model (no sum types, no trait-based plugin system).
- Runtime reflection and dynamic dispatch make the binary easier to reverse-engineer and patch, weakening tamper resistance.

### C++

**Pros:** Maximum performance, mature ecosystem, deep OS integration, no runtime overhead.

**Rejected because:**
- Memory safety is entirely the developer's responsibility. For a security-critical privileged process, the risk of memory corruption bugs is too high for a small team without dedicated security reviewers.
- Cross-platform build systems (CMake, Bazel) are significantly more complex than Cargo. Build reproducibility across five platforms would consume substantial engineering time.
- No ownership/borrow checker means concurrency bugs (data races in the watchdog, DNS resolver, and event reporter running simultaneously) are caught at runtime, not compile time.
- Modern C++ (C++20/23) closes some gaps, but the ecosystem tooling (package management, linting, formatting) is fragmented compared to Rust's unified Cargo/clippy/rustfmt.

### .NET (C# with NativeAOT)

**Pros:** Excellent developer productivity, strong type system, NativeAOT now produces small native binaries, good cross-platform story via .NET MAUI/runtime.

**Rejected because:**
- NativeAOT is mature on Windows and Linux but has significant limitations on macOS (no System Extension support) and is not viable on iOS (Apple prohibits JIT; AOT works but the ecosystem is thin).
- Android support via .NET MAUI adds a substantial runtime layer. The agent needs to be a lightweight native service, not a MAUI application.
- GC is still present even with NativeAOT (it's ahead-of-time compiled but still garbage collected). Same latency concerns as Go, though .NET's GC is more tunable.
- FFI to native OS APIs exists (P/Invoke, LibraryImport) but is less ergonomic than Rust's direct C-ABI compatibility.
- Would not share types with a Rust API. If the API were also .NET this would be a strong contender, but the vision document specifies Rust + Axum for the API.

### Hybrid: Rust core + platform-native apps (Swift, Kotlin, C#)

**Considered as:** Write the blocking engine in Rust, but write each platform's service layer in the native language (Swift daemon on macOS, Kotlin service on Android, etc.).

**Rejected because:**
- Multiplies the codebase by 5x for service lifecycle, configuration, watchdog, event reporting, and API communication -- all of which are platform-independent logic.
- The "thin shim" approach achieves the same OS integration with 10% of the native code. A 200-line Swift bridge to Network Extension is maintainable; a 5,000-line Swift service layer is a separate product.
- Debugging cross-language issues (Rust core called from Swift called from launchd) is harder than debugging a Rust binary that calls into a Swift bridge.

## Consequences

### What becomes easier

- **Single codebase for core logic.** Blocking engine, blocklist matching, event reporting, API client, and tamper detection are written once. Bug fixes and features propagate to all platforms simultaneously.
- **Shared types with API.** The `bb-core` crate is a dependency of both the agent and the API server. Enrollment structures, blocklist entries, and event schemas are defined once. Serialization mismatches are caught at compile time.
- **Memory safety guarantees.** The borrow checker eliminates entire classes of vulnerabilities (use-after-free, double-free, buffer overflows, data races) at compile time. For a privileged system service, this is a critical security property.
- **Small, self-contained binaries.** Rust produces small statically-linked binaries with no runtime dependencies. The agent can be distributed as a single file per platform.
- **Tamper resistance.** Compiled Rust binaries are harder to decompile and patch than managed-language bytecode. No reflection, no runtime type information by default.

### What becomes harder

- **Hiring.** Rust developers are less common than Go, C++, or C# developers. The team must either hire Rust specialists or invest in training. Mitigation: the platform shims are small enough that contributors familiar with Swift/Kotlin can work on those without deep Rust knowledge.
- **Compilation times.** Rust's compile times are significantly longer than Go's. A full clean build of the workspace across all platforms will take 10-20 minutes in CI. Mitigation: incremental builds are fast, and `cargo check` provides rapid feedback during development. CI caching (sccache) reduces rebuild times.
- **Learning curve for async Rust.** Tokio + async/await in Rust has sharp edges (pinning, lifetime issues in async contexts, `Send` bounds). The DNS resolver and API client are async-heavy. Mitigation: establish clear patterns early, use `tower` middleware for composable async services, and keep the blocking engine synchronous where possible.
- **Platform shim maintenance.** Each platform shim requires OS-specific expertise. When Apple changes the Network Extension API or Google updates VpnService, someone with platform knowledge must update the corresponding shim. Mitigation: shims are minimal (under 500 lines), well-tested, and isolated behind Rust traits.
- **Mobile binary size pressure.** While Rust binaries are small by default, pulling in `tokio` + `rustls` + `hickory-dns` on Android/iOS can push the binary to 8-12 MB. Mitigation: use `min-sized-release` profile, enable LTO, strip symbols, and consider `smol` as a lighter async runtime on mobile if tokio proves too heavy.

## Implementation Notes

### Build System

- Use a Cargo workspace at the repository root
- Platform-conditional compilation via `#[cfg(target_os)]` within shim crates
- CI builds all targets on every PR; platform-specific tests run on dedicated runners (macOS runner for apple targets, Windows runner for MSVC targets, Linux runner for musl targets, Android emulator for Android)
- Cross-compilation for Android and iOS uses `cross` or platform-specific toolchains in CI

### Binary Signing

- All release binaries are signed with the BetBlocker signing key
- macOS binaries are notarized with Apple
- Windows binaries are Authenticode signed
- Android libraries are signed within the APK signing process
- iOS libraries are signed within the Xcode archive process
- Self-hosted operators can configure their own signing authority (see ADR-006)

### Minimum Supported Rust Version (MSRV)

- Pin to stable Rust, updated quarterly
- No nightly features in production code
- CI tests against MSRV and latest stable

### Testing Strategy

- Unit tests for `bb-core`, `bb-dns`, `bb-net`, `bb-tamper` run on all platforms in CI
- Integration tests for platform shims require platform-specific CI runners
- Fuzz testing for DNS parser and blocklist matcher (using `cargo-fuzz`)
- Property-based testing for serialization roundtrips between agent and API (using `proptest`)
