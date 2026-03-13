# ADR-005: Tamper Resistance Architecture

## Status
Proposed

## Date
2026-03-12

## Context

BetBlocker's endpoint agent runs on devices owned by people who may be actively trying to disable it. This is fundamentally different from typical security software (which protects against external attackers) -- BetBlocker must resist the device's own administrator.

The threat model is:

1. **Casual user (low skill):** Tries to uninstall the app, stop the service, or change DNS settings. Expected to be deterred by standard service-level protections.
2. **Motivated user (medium skill):** Searches online for bypass methods, tries to kill processes, delete files, modify the HOSTS file, install a VPN, or use a different browser. Expected to be deterred by elevated protections.
3. **Technical user (high skill):** Has admin/root access, can inspect the binary, modify system configuration, install kernel modules, or use debugging tools. Expected to be significantly slowed (but not perfectly prevented -- no user-space software can fully resist a determined local admin with kernel access).

The architecture must be honest about what is achievable at each level and on each platform. Overpromising tamper resistance is worse than honestly communicating the protection level.

**Key constraint:** BetBlocker does not ship kernel-mode code in Phase 1. Kernel drivers (WFP callout driver, minifilter) are Phase 2 deliverables. Phase 1 tamper resistance operates entirely in user space, with OS-provided protections.

## Decision

### Layered Defense Model

Tamper resistance is implemented as concentric layers. Each layer can be defeated independently, but defeating all layers simultaneously is progressively harder.

```
Layer 5: Accountability (heartbeats, partner/authority alerts)
Layer 4: OS-level protections (service config, file permissions, platform APIs)
Layer 3: Self-healing (binary integrity, config restoration)
Layer 2: Watchdog (mutual process supervision)
Layer 1: Service-level protection (system service, no user-level stop)
```

An attacker must defeat Layer 1 before they can attempt Layer 2, and so on. Each layer buys time for the accountability layer (Layer 5) to detect and alert.

### Layer 1: System Service Protection

The agent runs as a system-level service that unprivileged users cannot stop, disable, or modify.

| Platform | Service Type | Stop Protection |
|----------|-------------|-----------------|
| Windows | Windows Service (LocalSystem) | Requires admin elevation to stop via `sc stop`. Service recovery options set to restart on failure (first: 0s, second: 5s, third: 30s). |
| macOS | launchd daemon (/Library/LaunchDaemons/) | Requires root to `launchctl unload`. `KeepAlive = true` auto-restarts. `Program` flag prevents user-level override. |
| Linux | systemd service (root) | Requires root to `systemctl stop`. `Restart=always`, `RestartSec=5`. `ProtectSystem=strict` for read-only filesystem. |
| Android | Foreground Service + Device Admin | Cannot be stopped from recent apps. Device Admin prevents uninstall without admin deactivation. |
| iOS | Network Extension (managed) | Cannot be killed by user. MDM profile prevents removal on authority tier. |

### Layer 2: Watchdog (Mutual Process Supervision)

Two processes run simultaneously: the main agent and a lightweight watchdog. Each monitors the other.

**Design:**

```
+------------------+         heartbeat          +------------------+
|   Main Agent     | <------------------------> |    Watchdog      |
|  (bb-agent)      |   shared memory / pipe     |  (bb-watchdog)   |
|                  |                            |                  |
|  Blocking engine |         If watchdog dies:  |  If agent dies:  |
|  DNS resolver    |         Agent restarts it  |  Watchdog        |
|  Event reporter  |                            |  restarts agent  |
|  API client      |                            |                  |
+------------------+                            +------------------+
```

**Communication:** The agent and watchdog communicate via a platform-appropriate IPC mechanism:

- **Windows:** Named pipe (`\\.\pipe\betblocker-watchdog`)
- **macOS/Linux:** Unix domain socket (`/var/run/betblocker/watchdog.sock`)
- **Android:** Bound service with AIDL interface
- **iOS:** Not applicable (iOS does not support background watchdog processes; relies on Network Extension's built-in lifecycle management)

**Heartbeat protocol:**

1. Every 5 seconds, the agent sends a heartbeat to the watchdog containing: timestamp, PID, binary hash, blocklist version.
2. Every 5 seconds, the watchdog sends a heartbeat to the agent containing: timestamp, PID, watchdog binary hash.
3. If either process misses 3 consecutive heartbeats (15 seconds), the surviving process:
   a. Logs a tamper detection event.
   b. Attempts to restart the dead process from a known-good binary path.
   c. If restart fails 3 times, sends a high-priority tamper alert to the API (if network available).

**Anti-kill coordination:** To kill both processes simultaneously, an attacker must:
- Identify both PIDs (the watchdog has a randomized process name, not "betblocker-watchdog")
- Kill both within the 5-second heartbeat window
- Prevent the OS service manager from restarting them (Layer 1)

This is achievable for a technical user with admin access, but it requires specific knowledge and deliberate action, which is the point: it converts impulsive bypass attempts into deliberate ones that take time and leave evidence.

### Layer 3: Self-Healing (Binary and Config Integrity)

**Binary integrity:**

1. On startup, the agent computes the SHA-256 hash of its own binary and compares it against an expected hash.
2. The expected hash is stored in two places: (a) embedded in the watchdog binary during build, and (b) in the signed configuration received from the API.
3. If the hash mismatches, the agent:
   a. Logs a binary tampering event.
   b. Attempts to restore from a cached signed copy (stored in a protected directory).
   c. If restoration fails, enters "degraded mode": continues blocking with existing blocklist but refuses to sync new configuration until integrity is restored.

**Configuration integrity:**

1. The enrollment configuration (tier, permissions, blocklist signing key) is encrypted at rest using a hardware-bound key (see below).
2. On every config load, the agent verifies the configuration's signature (signed by the API during enrollment).
3. If the config is corrupted or missing, the agent enters "safe mode": blocks everything in the seed list, sends a tamper alert, and waits for re-enrollment.

**Hardware-bound key storage:**

The agent's private key (device certificate private key) and configuration encryption key are stored in hardware-backed keystores:

| Platform | Keystore | Key Properties |
|----------|----------|----------------|
| Windows | TPM 2.0 via Platform Crypto Provider | Non-exportable, requires PCR binding for TPM-equipped machines. Fallback: DPAPI with LSA protection. |
| macOS | Secure Enclave via Keychain Services | `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`, non-exportable. |
| Linux | TPM 2.0 via tpm2-tss. Fallback: LUKS-encrypted keyfile with root-only permissions. | Key sealed to PCR values where TPM is available. |
| Android | Android Keystore (StrongBox where available) | `setIsStrongBoxBacked(true)`, `setUserAuthenticationRequired(false)`. |
| iOS | Secure Enclave via Keychain Services | Same as macOS, device-bound, non-migratable. |

**Fallback chain:** TPM/Secure Enclave -> software keystore with OS protection (DPAPI/Keychain) -> encrypted file with restrictive permissions. The agent reports its key storage level in heartbeats so the API knows the device's tamper resistance capability.

### Layer 4: OS-Level Protections

These are platform-specific protections that leverage OS security features. Phase 1 uses only user-space protections. Phase 2 adds kernel-level protections where available.

#### Windows

**Phase 1 (User Space):**
- Service runs as LocalSystem with `SERVICE_FAILURE_ACTIONS` set to restart.
- Agent files in `C:\Program Files\BetBlocker\` with ACL: SYSTEM=FullControl, Administrators=ReadExecute, Users=ReadExecute. Deny delete for non-SYSTEM principals.
- Registry keys for service configuration protected with restrictive ACLs.
- DNS configuration monitoring: agent polls `GetAdaptersInfo()` every 30 seconds, reverts unauthorized DNS changes.

**Phase 2 (Kernel Level):**
- WFP callout driver: persists DNS redirection rules in the Windows Filtering Platform. If the agent process dies, WFP rules continue to block. Requires WHQL-signed driver.
- Kernel minifilter: prevents deletion or modification of agent files, even by administrators. Requires WHQL-signed driver.
- Protected Process Light (PPL): if the agent qualifies for ELAM (Early Launch Anti-Malware), it runs as a PPL process that even admin-level processes cannot terminate.

#### macOS

**Phase 1 (User Space):**
- launchd daemon with `KeepAlive`, `RunAtLoad`, `AbandonProcessGroup`.
- Agent files in `/Library/Application Support/BetBlocker/` with `root:wheel` ownership, `755` permissions.
- Configuration in `/Library/Preferences/com.betblocker.agent.plist` with root-only write.
- DNS monitoring via `SCDynamicStore` notifications.

**Phase 2 (Kernel Level, via System Extension):**
- System Extension (Network Extension + Endpoint Security): runs in a separate sandbox, survives agent process death, requires user approval + reboot to remove.
- Endpoint Security framework: monitors file modifications to agent files, process kills targeting agent, and authorization events.
- On Apple Silicon: Signed System Volume means the agent cannot be placed on the boot volume, but System Extension registration is persistent.

#### Linux

**Phase 1 (User Space):**
- systemd service with `Restart=always`, `ProtectSystem=strict`, `ProtectHome=true`, `PrivateTmp=true`, `ReadWritePaths=/var/lib/betblocker`.
- Agent binary at `/usr/lib/betblocker/` with `root:root`, `755`, `chattr +i` (immutable attribute).
- nftables rules for DNS redirection with `nft` hooks that require root to modify.

**Phase 2 (Kernel Level):**
- AppArmor profile: confines the agent's access but also protects agent files from other processes. Denies `ptrace`, signal sending, and file modification for non-agent processes.
- SELinux policy (for RHEL/Fedora): custom policy module that protects agent files and processes.
- eBPF programs (experimental): lightweight DNS interception at the kernel level that persists if the agent process dies.

#### Android

**Phase 1:**
- Foreground Service with persistent notification (required by Android for long-running services).
- Device Administrator enrollment: prevents app uninstall without deactivating admin first. Deactivation triggers tamper alert.
- VpnService for DNS interception: user can disconnect the VPN, but the agent detects this within seconds and prompts to reconnect.
- `android:persistent="true"` in manifest (only effective for system apps or Device Owner).

**Phase 2:**
- Device Owner enrollment (via QR code provisioning): highest level of control. Can silently re-enable VPN, prevent uninstall, and restrict settings access.
- Samsung Knox: Managed VPN that user cannot disconnect, app protection that prevents force-stop.
- Work profile: isolates the VPN and blocking in a managed profile that requires admin consent to remove.

#### iOS

**Phase 1:**
- Network Extension (NEDNSProxyProvider): operates as a system-level DNS proxy. User can disable it in Settings > VPN & Network, but the agent detects disconnection via NETunnelProviderManager notifications and prompts reconnection.
- Content Blocker extension: provides backup blocking via Safari Content Blocker rules.

**Phase 2:**
- MDM profile: for authority tier, the Network Extension is deployed via MDM and cannot be removed without MDM authority approval. This maps directly to the enrollment authority model.
- Supervised device mode: for institutional deployments, prevents users from modifying VPN/DNS settings entirely.

### Layer 5: Accountability (Heartbeats and Alerts)

This is the ultimate tamper resistance layer: even if all technical protections are defeated, the partner/authority is notified.

**Heartbeat protocol:**

```rust
pub struct Heartbeat {
    pub device_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub agent_version: String,
    pub blocklist_version: u64,
    pub protection_status: ProtectionStatus,
    pub active_plugins: Vec<String>,
    pub key_storage_level: KeyStorageLevel,
    pub last_block_event: Option<DateTime<Utc>>,
    pub integrity_check: IntegrityReport,
}

pub struct ProtectionStatus {
    pub service_running: bool,
    pub watchdog_running: bool,
    pub dns_interception_active: bool,
    pub vpn_connected: bool,         // Android/iOS
    pub binary_integrity_ok: bool,
    pub config_integrity_ok: bool,
}
```

**Heartbeat intervals and alert thresholds:**

| Tier | Heartbeat Interval | Alert After Missed | Alert Recipient |
|------|-------------------|--------------------|-----------------|
| Self | 1 hour | Configurable (default: 24h) | User (optional) |
| Partner | 30 minutes | 2 hours | Partner + user |
| Authority | 15 minutes | 1 hour | Authority + audit log |

**Tamper alert escalation:**

1. **Level 1 (Informational):** DNS settings changed, VPN disconnected, extension removed. Auto-remediated if possible. Logged.
2. **Level 2 (Warning):** Agent process killed and restarted by watchdog, binary hash mismatch detected and repaired, configuration corruption detected. Logged + partner/authority notified.
3. **Level 3 (Critical):** Both agent and watchdog killed, service uninstalled, device admin deactivated. Partner/authority notified immediately. If authority tier: compliance violation flagged.

## Alternatives Considered

### Kernel-Mode Agent from Phase 1

**Pros:** Maximum tamper resistance. A kernel driver is extremely difficult to remove or bypass without admin access AND technical knowledge.

**Rejected for Phase 1 because:**
- Kernel drivers require platform-specific signing (WHQL for Windows, Apple notarization for macOS). This is a multi-month process per platform.
- Kernel bugs are catastrophic (BSOD, kernel panic). Shipping kernel code before the user-space agent is battle-tested is reckless.
- The Phase 1 protection level (service + watchdog + integrity + accountability) is sufficient to deter casual and motivated users. Kernel protection is added in Phase 2 for the technical user threat.

### No Watchdog (Rely on OS Service Manager)

**Pros:** Simpler. OS service managers (systemd, launchd, SCM) already restart crashed services.

**Rejected because:**
- OS service managers restart on crash, not on kill. A `taskkill /F` or `kill -9` may not trigger the "failure" recovery action on all platforms (Windows SCM in particular does not always treat `TerminateProcess` as a "failure").
- The watchdog provides mutual integrity checking, not just liveness. It verifies the agent's binary hash, which the OS service manager cannot do.
- The watchdog's randomized process name makes it harder to identify and target.

### Hardware Attestation (Remote Attestation via TPM)

**Considered for:** Proving to the API that the agent binary is unmodified and running on genuine hardware.

**Deferred because:**
- Remote attestation requires TPM 2.0, which is available on most modern PCs but not on Android (no standardized API) or iOS (no TPM, Secure Enclave is not attestable remotely in the same way).
- The attestation infrastructure (Attestation CA, quote verification) is complex. Phase 1 uses simpler integrity checks (binary hash in heartbeat).
- Will be revisited for Phase 2 authority tier, where compliance requirements may mandate hardware attestation.

### Virtualization-Based Security (VBS)

**Considered for Windows:** Running the agent in a VBS-protected enclave (Hyper-V isolated).

**Deferred because:**
- VBS requires Windows 10/11 Enterprise or Pro with Hyper-V enabled. Many consumer devices do not meet these requirements.
- The development effort for VBS integration is disproportionate to the threat model. Most Windows users in the target audience are casual or motivated users, not technical users who can defeat kernel-level protections.

## Consequences

### What becomes easier

- **Honest communication about protection levels.** The layered model allows us to clearly communicate what each tier provides: "Self-enrolled devices resist casual bypass. Partner-enrolled devices alert your partner if tampering is detected. Authority-enrolled devices use maximum OS-level protections."
- **Progressive hardening.** Phase 1 ships meaningful protection without kernel code. Phase 2 adds kernel protections incrementally per platform, each on its own timeline.
- **Cross-platform consistency.** The five-layer model applies to all platforms, even if the specific mechanisms differ. The API treats all platforms uniformly: heartbeats, alerts, and integrity reports have the same schema.

### What becomes harder

- **Testing tamper resistance.** Each protection layer must be tested by actually attempting to bypass it, on each platform. This requires dedicated QA environments with admin access and platform expertise. Mitigation: automated tamper resistance test suites that run in CI (start agent, kill it, verify restart; modify binary, verify detection).
- **False positives.** Overzealous tamper detection can flag legitimate system operations (OS updates that modify DNS, antivirus quarantining the agent binary, corporate VPN that changes network routes). Mitigation: maintain a known-safe-operations allowlist, and ensure tamper alerts are logged but not punitive in self-enrolled tier.
- **User trust.** Users may distrust software that resists being removed. The UX must clearly communicate: (a) the user chose to enroll, (b) unenrollment is always possible via the defined policy (time delay for self, partner approval for partner), and (c) the protection exists to help them, not control them.

## Implementation Notes

### Phase 1 Tamper Resistance Checklist

- [ ] System service registration on all 5 platforms with auto-restart
- [ ] Watchdog process with mutual heartbeat (5s interval, 15s timeout)
- [ ] Randomized watchdog process name (generated at install time)
- [ ] Binary integrity check on startup and every 30 minutes
- [ ] Configuration encryption with hardware-bound key (TPM/Keychain/Keystore)
- [ ] Fallback to software keystore where hardware is unavailable
- [ ] DNS configuration monitoring and reversion (30s poll)
- [ ] File permission lockdown (SYSTEM/root only write)
- [ ] Heartbeat reporting with protection status
- [ ] Tamper alert escalation (3 levels)
- [ ] Seed blocklist fallback when config is corrupted

### Phase 2 Tamper Resistance Additions

- [ ] WFP callout driver (Windows) -- WHQL signing process
- [ ] Kernel minifilter (Windows) -- file protection
- [ ] System Extension (macOS) -- persistent network filtering
- [ ] Endpoint Security (macOS) -- process and file monitoring
- [ ] AppArmor/SELinux profiles (Linux) -- MAC policies
- [ ] Device Owner provisioning (Android) -- maximum device control
- [ ] Knox integration (Samsung Android) -- managed VPN
- [ ] MDM profile deployment (iOS) -- non-removable Network Extension
- [ ] Hardware attestation for authority tier (TPM-based)
