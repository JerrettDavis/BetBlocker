# BetBlocker Security Threat Model

**Date:** 2026-03-12
**Status:** Draft
**Classification:** Internal -- Security Sensitive
**Authors:** JD + Claude
**Review Cadence:** Quarterly, or on any architecture change

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Threat Actors](#2-threat-actors)
3. [Attack Surfaces](#3-attack-surfaces)
4. [STRIDE Threat Analysis](#4-stride-threat-analysis)
5. [Privacy Threat Model](#5-privacy-threat-model)
6. [Anti-Bypass Matrix](#6-anti-bypass-matrix)
7. [Supply Chain Security](#7-supply-chain-security)
8. [Compliance Considerations](#8-compliance-considerations)
9. [Risk Summary and Prioritization](#9-risk-summary-and-prioritization)
10. [Appendix: Threat Registry](#10-appendix-threat-registry)

---

## 1. Introduction

### 1.1 Purpose

This document is the comprehensive security threat model for the BetBlocker platform. It identifies threat actors, enumerates attack surfaces, analyzes threats using the STRIDE methodology, and prescribes specific mitigations with implementation timelines.

### 1.2 Why This Threat Model Is Different

BetBlocker inverts the standard security model. In most applications, the device owner is the defender and external parties are the attackers. In BetBlocker:

- **The device owner is often the primary threat actor.** A self-enrolled user experiencing a gambling urge has physical access to the device, administrative credentials to the operating system, and a powerful motivation to bypass protections.
- **The "attacker" has root access.** On most platforms, the user can escalate to administrator. The agent must survive in a hostile environment where the adversary controls the machine.
- **Bypass is the primary threat, not breach.** The most damaging outcome is not data theft but the user successfully reaching a gambling site during a moment of vulnerability.
- **Security must be compassionate.** The person trying to bypass the system is also the person the system is trying to protect. Aggressive lockdown that damages the user's device or data is unacceptable.

This tension -- maximum tamper resistance with minimum collateral harm -- is the central engineering challenge and shapes every mitigation in this document.

### 1.3 Scope

| In Scope | Out of Scope |
|----------|-------------|
| Endpoint agent (all 5 platforms) | Physical device theft (not a gambling-specific threat) |
| Central API | Social engineering of BetBlocker staff |
| Web platform | Attacks against upstream cloud providers |
| Background worker | Hardware implant attacks |
| Data stores | Quantum computing attacks on cryptography |
| Communication channels | Legal/regulatory compliance enforcement |
| Self-hosted deployments | |
| Supply chain and update mechanism | |

### 1.4 Data Classification

| Classification | Examples | Protection Level |
|---------------|----------|-----------------|
| **Critical** | Enrollment credentials, API signing keys, mTLS certificates | Hardware-bound encryption, HSM for server-side keys |
| **Sensitive** | Account credentials, partner relationships, blocked-attempt logs | Encrypted at rest, encrypted in transit, access-controlled |
| **Internal** | Blocklist data, aggregated analytics, platform configuration | Encrypted in transit, standard access controls |
| **Public** | Marketing content, open-source code, community blocklist feed | Integrity protection (signatures), no confidentiality requirement |

---

## 2. Threat Actors

### TA-1: Self-Enrolled User (Bypass)

- **Description:** An individual who enrolled themselves during a period of resolve and now wants to access gambling during a moment of weakness.
- **Motivation:** Immediate gratification; gambling urge.
- **Sophistication:** Low to moderate. Will search "how to uninstall BetBlocker" or "bypass DNS filter." Some will be technically proficient.
- **Access:** Full physical access to device. Likely has or can obtain administrator/root credentials. Owns the device.
- **Constraints:** Time-delay unenrollment exists. Notification system creates social accountability even without a partner.
- **Frequency:** This is the most common threat actor by volume. Every enrolled device faces this threat.

### TA-2: Partner-Enrolled User (Bypass)

- **Description:** A user whose device was enrolled by an accountability partner (spouse, therapist, sponsor). The user wants to bypass blocking without the partner knowing.
- **Motivation:** Same as TA-1, plus desire to avoid social consequences.
- **Sophistication:** Moderate to high. Willingness to invest significant time and effort. May hire someone or use detailed online guides.
- **Access:** Full physical access. May or may not have admin credentials (partner may have set up the device).
- **Constraints:** Cannot unenroll without partner approval. Partner receives tamper alerts. Must bypass silently to avoid detection.
- **Frequency:** High. This is the core use case for the partner tier.

### TA-3: Court-Ordered User (Bypass)

- **Description:** A user enrolled by a court order, probation program, or institutional mandate. Bypass has legal consequences.
- **Motivation:** Extremely high. Addiction plus resentment of external control.
- **Sophistication:** Potentially very high. May seek professional help to bypass. Legal stakes justify significant investment.
- **Access:** Full physical access. Device may be somewhat locked down by the institution, or may be the user's personal device.
- **Constraints:** Compliance reporting means bypasses are eventually detected. Legal consequences are severe. But the user may be desperate.
- **Frequency:** Lower volume than TA-1/TA-2, but the highest consequence per incident.

### TA-4: External Attacker (Platform)

- **Description:** Standard web application attacker targeting the BetBlocker platform itself.
- **Motivation:** Data theft (PII, partner relationships), platform disruption, ransomware, ideological (pro-gambling industry).
- **Sophistication:** Low (script kiddie) to high (organized crime, hired by gambling operators).
- **Access:** Network access to public API and web platform. No physical device access.
- **Constraints:** Standard network security controls. No insider access.
- **Frequency:** Continuous. Automated scanning is constant. Targeted attacks are less frequent but possible given the sensitive population served.

### TA-5: Malicious Accountability Partner (Surveillance Abuse)

- **Description:** A partner who uses BetBlocker as a surveillance or control tool in an abusive relationship. This is not about gambling -- it is about monitoring and controlling the other person.
- **Motivation:** Control, jealousy, domestic abuse.
- **Sophistication:** Low to moderate. Uses the platform's legitimate features in an unintended way.
- **Access:** Full partner dashboard access. Receives all reports and alerts configured for the enrollment.
- **Constraints:** BetBlocker only blocks gambling and only reports gambling-related events. But even metadata (device online/offline, tamper attempts, blocked domains) can be abused.
- **Frequency:** Difficult to quantify. Domestic abuse is prevalent. Any tool that gives one person visibility into another's device usage will be misused.

### TA-6: Rogue Self-Hosted Operator (Data Harvesting)

- **Description:** Someone who deploys a self-hosted BetBlocker instance and offers it to others, but modifies the server to harvest user data, expand surveillance beyond gambling, or inject malicious blocklist entries.
- **Motivation:** Data harvesting, surveillance, control over a vulnerable population.
- **Sophistication:** High. Capable of modifying server-side code and deploying custom builds.
- **Access:** Full control of the server-side infrastructure. May also distribute modified agent binaries.
- **Constraints:** If users download the official agent, it will validate server certificates. But a rogue operator controls the enrollment process and can instruct users to install a modified agent.
- **Frequency:** Low, but high impact. The self-hosted model inherently enables this.

### TA-7: Supply Chain Attacker

- **Description:** An attacker who compromises the BetBlocker build pipeline, a dependency, or the update mechanism to distribute malicious code to all enrolled devices.
- **Motivation:** Mass compromise of devices. BetBlocker agents run as SYSTEM/root -- this is an extremely high-value target.
- **Sophistication:** Very high. Nation-state or organized crime level.
- **Access:** Indirect -- through compromised dependencies, CI/CD systems, or signing keys.
- **Constraints:** Code signing, reproducible builds, and update verification limit the blast radius.
- **Frequency:** Rare, but catastrophic if successful. The agent running as SYSTEM/root on every enrolled device makes this one of the highest-impact scenarios.

### TA-8: Nation-State Actor

- **Description:** State-sponsored attacker targeting BetBlocker infrastructure or users. Relevant for the institutional tier (court systems, government programs).
- **Motivation:** Surveillance of specific individuals, disruption of judicial programs, intelligence gathering.
- **Sophistication:** Maximum. Zero-day exploits, hardware implants, insider recruitment.
- **Access:** Potentially any -- network, physical, insider, supply chain.
- **Constraints:** BetBlocker is unlikely to be a primary target for nation-states. But users in institutional programs may be individually targeted, and the platform becomes a vector.
- **Frequency:** Very low. Included for completeness and to inform the institutional tier's security requirements.

---

## 3. Attack Surfaces

### 3.1 Endpoint Agent

| Surface | Description | Exposed To |
|---------|------------|-----------|
| Agent process | Running service/daemon that can be killed, suspended, or debugged | TA-1, TA-2, TA-3 |
| Agent binary | Executable files on disk that can be replaced, deleted, or modified | TA-1, TA-2, TA-3 |
| Agent configuration | Enrollment credentials, blocklist cache, policy settings | TA-1, TA-2, TA-3 |
| DNS resolution path | Local resolver that can be bypassed by changing DNS settings, using DoH/DoT, or modifying HOSTS file | TA-1, TA-2, TA-3 |
| Network path | Network interfaces, VPN tunnels, proxy settings, Tor, tethering | TA-1, TA-2, TA-3 |
| Operating system | Safe mode, recovery mode, OS reinstall, factory reset, boot from USB | TA-1, TA-2, TA-3 |
| System clock | Time manipulation to accelerate time-delay unenrollment | TA-1, TA-2, TA-3 |
| Inter-process communication | Watchdog communication, service control interfaces | TA-1, TA-2, TA-3 |
| Update mechanism | Auto-update channel that could be hijacked to deliver malicious binaries | TA-7 |
| Memory | Runtime memory that could be inspected or modified with debuggers | TA-2, TA-3 |

### 3.2 Central API

| Surface | Description | Exposed To |
|---------|------------|-----------|
| Authentication endpoints | Login, token refresh, password reset | TA-4 |
| Enrollment endpoints | Create, modify, unenroll -- policy enforcement is here | TA-1, TA-2, TA-3, TA-4 |
| Device communication | Blocklist sync, heartbeat, event ingestion | TA-4, TA-7 |
| Partner/authority endpoints | Approval workflows, report access | TA-4, TA-5 |
| Federated report ingestion | Accepts domain reports from agents | TA-4, TA-6 |
| Admin endpoints | Blocklist management, platform configuration | TA-4 |
| Billing endpoints | Stripe webhook receiver, subscription management | TA-4 |

### 3.3 Web Platform

| Surface | Description | Exposed To |
|---------|------------|-----------|
| Authentication UI | Login, registration, password reset flows | TA-4 |
| Partner dashboard | Managed device list, approval queue, reports | TA-4, TA-5 |
| Authority dashboard | Compliance views, audit logs, bulk management | TA-4 |
| Admin panel | Blocklist management, platform analytics | TA-4 |
| Client-side code | JavaScript bundles that could be tampered with (CDN compromise) | TA-4, TA-7 |

### 3.4 Communication Channels

| Surface | Description | Exposed To |
|---------|------------|-----------|
| Agent-to-API (mTLS) | Blocklist sync, heartbeat, event reporting | TA-4, TA-8 |
| Browser-to-API (TLS) | Web platform API calls | TA-4 |
| Federated blocklist feed | Community blocklist distribution | TA-6, TA-7 |
| Push notifications | Tamper alerts, unenrollment notifications | TA-4 |

### 3.5 Data Stores

| Surface | Description | Exposed To |
|---------|------------|-----------|
| PostgreSQL | Account data, enrollment records, partner relationships | TA-4, TA-6, TA-8 |
| Redis | Session data, real-time device status | TA-4 |
| TimescaleDB | Event analytics, blocked-attempt history | TA-4, TA-5, TA-6 |
| Local blocklist cache | On-device blocklist copy | TA-1, TA-2, TA-3 |

---

## 4. STRIDE Threat Analysis

### 4.1 Spoofing

#### T-001: Agent Identity Spoofing

- **Category:** Spoofing
- **Description:** An attacker crafts a fake agent that impersonates a legitimate enrolled device to send false heartbeats, false "all clear" status reports, or to receive blocklist data without being a real agent.
- **Threat Actor:** TA-2, TA-3 (send fake heartbeats to appear compliant while using a different, unblocked device)
- **Attack Surface:** Agent-to-API communication
- **Likelihood:** Medium
- **Impact:** High -- undermines the entire compliance model for partner and authority tiers.
- **Risk Rating:** High
- **Mitigation:**
  - mTLS with per-device client certificates generated at enrollment time and bound to device hardware identifiers (TPM, Secure Enclave, Keystore).
  - Server validates that the device certificate matches the enrolled device's hardware fingerprint.
  - Heartbeat payloads include signed device attestation (Windows: TPM attestation, macOS: DeviceCheck, Android: SafetyNet/Play Integrity, iOS: DeviceCheck/App Attest).
  - Anomaly detection: flag devices whose heartbeat metadata (OS version, hardware model, network characteristics) changes unexpectedly.
- **Residual Risk:** Low. Hardware-bound certificates are extremely difficult to extract. Emulation attacks remain theoretically possible but require significant sophistication.
- **Phase:** Phase 1 (mTLS + device certs), Phase 2 (hardware attestation, anomaly detection)

#### T-002: Account Takeover via Credential Theft

- **Category:** Spoofing
- **Description:** Attacker gains access to a user's account credentials through phishing, credential stuffing, or data breach re-use.
- **Threat Actor:** TA-1 (take over own partner's account to approve own unenrollment), TA-4
- **Attack Surface:** Authentication endpoints, web platform
- **Likelihood:** High -- credential stuffing attacks are constant.
- **Impact:** Critical -- if a self-enrolled user takes over their partner's account, they can approve their own unenrollment. If an external attacker takes over any account, they gain access to sensitive enrollment and reporting data.
- **Risk Rating:** Critical
- **Mitigation:**
  - Enforce strong password requirements (minimum 12 characters, check against breached password databases via k-anonymity/HaveIBeenPwned API).
  - Mandatory MFA for partner and authority accounts. Strongly recommended for all accounts.
  - Rate limiting on authentication endpoints (progressive delays, account lockout after N failures).
  - Session binding (tie session to IP range and user-agent; challenge on significant change).
  - Credential stuffing detection (monitor for distributed low-rate attempts across many accounts).
  - Separate authentication contexts: a user account and their partner account must never share credentials, sessions, or recovery mechanisms.
  - For partner-enrolled unenrollment approval: require re-authentication with MFA, even within an active session.
- **Residual Risk:** Medium. MFA significantly reduces risk, but SIM-swapping and sophisticated phishing (real-time MFA relay) remain threats. Hardware security keys (WebAuthn/FIDO2) for authority-tier accounts reduce this further.
- **Phase:** Phase 1 (passwords, rate limiting, MFA), Phase 2 (WebAuthn, anomaly detection), Phase 3 (hardware keys for authority tier)

#### T-003: Partner Invitation Spoofing

- **Category:** Spoofing
- **Description:** Attacker intercepts or forges a partner invitation to establish themselves as someone's accountability partner without authorization.
- **Threat Actor:** TA-4, TA-5
- **Attack Surface:** Web platform, partner invitation flow
- **Likelihood:** Low
- **Impact:** High -- unauthorized partner gains visibility into the user's gambling-related device activity and can block or delay unenrollment.
- **Risk Rating:** Medium
- **Mitigation:**
  - Partner invitations use cryptographically random, single-use, time-limited tokens (expire after 48 hours).
  - Invitation acceptance requires the invitee to authenticate with their own account (no anonymous acceptance).
  - Both parties receive confirmation notifications with the identity of the linked partner.
  - Users can view and revoke partner relationships at any time from their dashboard (for self-enrolled tier; partner-enrolled tier requires the existing partner's consent to change partners).
- **Residual Risk:** Low.
- **Phase:** Phase 1

### 4.2 Tampering

#### T-004: Agent Binary Replacement

- **Category:** Tampering
- **Description:** User replaces the agent binary with a modified version that allows gambling traffic, or with a no-op binary that does nothing.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Agent binary on disk
- **Likelihood:** Medium -- requires admin access but there are tutorials for this kind of thing.
- **Impact:** Critical -- complete bypass of all blocking.
- **Risk Rating:** Critical
- **Mitigation:**
  - **Binary integrity validation:** Agent validates its own hash on startup and periodically (every 60 seconds) against a signed manifest.
  - **Watchdog mutual supervision:** Primary agent and watchdog process each monitor the other's binary integrity and running state. Each runs under a different service account where possible.
  - **Kernel-level file protection:** Windows kernel minifilter prevents modification/deletion of agent files. macOS Endpoint Security framework monitors file events. Linux immutable file attributes (`chattr +i`) plus AppArmor/SELinux policies.
  - **Self-healing:** If tampering detected, agent restores itself from a cached, signed copy stored in a protected location. On failure, triggers a tamper alert to the API before the compromised binary can suppress it.
  - **Tamper alert race condition:** The watchdog sends the tamper alert, not the primary agent. The watchdog is a separate binary in a separate protected location.
- **Residual Risk:** Medium. A sophisticated attacker with kernel-level access can disable file protection mechanisms. On platforms without kernel-level file protection (Android without Knox, standard Linux), the residual risk is higher.
- **Phase:** Phase 1 (hash validation, watchdog), Phase 2 (kernel-level file protection)

#### T-005: Agent Configuration Tampering

- **Category:** Tampering
- **Description:** User modifies the agent's configuration to change the blocklist, disable reporting, alter the enrollment tier, or modify the API endpoint.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Agent configuration files on disk
- **Likelihood:** Medium
- **Impact:** High -- could disable blocking, suppress tamper alerts, or redirect the agent to a fake API.
- **Risk Rating:** High
- **Mitigation:**
  - **Configuration encryption:** All configuration is encrypted with a key bound to device hardware (TPM on Windows, Secure Enclave on macOS/iOS, Keystore on Android, TPM or filesystem key on Linux).
  - **Configuration signing:** Configuration includes a server-signed integrity tag. Agent rejects any configuration that fails signature verification.
  - **API endpoint pinning:** The API endpoint and its certificate are embedded in the binary at build time (or at enrollment time) and cannot be changed by configuration alone. Re-enrollment is required to change API endpoints.
  - **Configuration stored in protected location:** Same file protection as the binary itself (kernel minifilter, immutable attributes, etc.).
- **Residual Risk:** Medium. If the hardware key store is compromised (e.g., by extracting keys from a rooted device), configuration can be decrypted and re-encrypted. Hardware attestation at the API level (T-001) provides a second layer of defense.
- **Phase:** Phase 1 (encryption, signing), Phase 2 (hardware binding, file protection)

#### T-006: Blocklist Cache Tampering

- **Category:** Tampering
- **Description:** User modifies the local blocklist cache to remove gambling domains, allowing access while the agent appears to be running normally.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Local blocklist cache on disk
- **Likelihood:** Medium
- **Impact:** Critical -- selective bypass of specific gambling sites while the system appears functional.
- **Risk Rating:** Critical
- **Mitigation:**
  - **Blocklist signing:** Every blocklist version is cryptographically signed by the API. Agent validates signature before loading.
  - **Blocklist version tracking:** API knows what version each device should have. If an agent reports blocking with a stale or invalid version, flag for investigation.
  - **Periodic re-download:** Agent periodically re-downloads the full blocklist (not just deltas) to detect tampering of the cached copy.
  - **In-memory blocklist:** After validation, the blocklist is held in memory. Disk cache is only for persistence across restarts. Tampering the disk cache causes a re-download on next start, not a bypass.
  - **HOSTS file fallback:** Even if the primary blocklist is tampered with, the HOSTS file plugin maintains a separate copy that blocks the highest-risk domains.
- **Residual Risk:** Low. The attacker would need to tamper with both the in-memory blocklist and the HOSTS file simultaneously, and suppress the version mismatch alert to the API.
- **Phase:** Phase 1

#### T-007: DNS Configuration Bypass (HOSTS File, DoH/DoT)

- **Category:** Tampering
- **Description:** User changes the system DNS configuration to bypass the agent's local resolver. This includes editing the HOSTS file to point gambling domains to known IPs, configuring a non-local DNS server, using DNS-over-HTTPS (DoH) or DNS-over-TLS (DoT) to bypass interception, or configuring applications to use their own DNS.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** DNS resolution path, network configuration
- **Likelihood:** High -- this is the most commonly searched bypass technique for DNS-based filters.
- **Impact:** Critical -- complete bypass of DNS-based blocking.
- **Risk Rating:** Critical
- **Mitigation:**
  - **HOSTS file monitoring:** Agent monitors the HOSTS file for changes. If gambling-related entries are detected, revert them. If non-agent changes are detected, alert.
  - **DNS configuration lockdown:** Agent monitors and enforces DNS settings. Revert any changes to the system DNS configuration that point away from the local resolver.
  - **DoH/DoT interception:** Agent becomes the device's DoH/DoT provider. Block or redirect known public DoH/DoT endpoints (Cloudflare 1.1.1.1, Google 8.8.8.8, etc.) at the network level using platform hooks (WFP, NetworkExtension, iptables).
  - **Platform network hooks:** Use WFP (Windows), NetworkExtension (macOS/iOS), VpnService (Android), iptables/nftables (Linux) to intercept all DNS traffic regardless of the application's DNS configuration. This is the critical control -- all DNS must flow through the agent.
  - **Application-level DNS:** Some applications (Chrome, Firefox) have built-in DoH. Agent monitors browser settings and disables built-in DoH, or configures it to point to the agent's DoH endpoint.
  - **IP-based blocking:** For the most critical gambling domains, maintain an IP blocklist as a fallback. Even if DNS is bypassed, direct IP connections to known gambling IPs are blocked at the network filter level.
- **Residual Risk:** Medium. Novel DoH endpoints or encrypted DNS protocols not yet known to the agent could bypass detection. IP-based blocking is an imperfect fallback because gambling sites frequently change IPs and use CDNs.
- **Phase:** Phase 1 (DNS monitoring, HOSTS monitoring, platform hooks), Phase 2 (DoH/DoT interception, IP-based blocking, browser DoH management)

#### T-008: Blocklist Poisoning via Federated Reports

- **Category:** Tampering
- **Description:** A malicious actor submits false federated reports to get legitimate domains added to the blocklist (denial of service against non-gambling sites) or to flood the review queue to delay processing of real gambling domains.
- **Threat Actor:** TA-4, TA-6
- **Attack Surface:** Federated report ingestion endpoint
- **Likelihood:** Medium
- **Impact:** Medium -- false positives damage user trust; queue flooding delays real blocking.
- **Risk Rating:** Medium
- **Mitigation:**
  - **Reputation system:** Weight reports by the reputation of the reporting agent/account. New accounts have low weight. Established accounts with consistent reporting history have high weight.
  - **Threshold for promotion:** A domain is only promoted to the blocklist review queue after reports from multiple independent agents (configurable threshold, e.g., 5 unique agents).
  - **Automated pre-classification:** Before human review, run automated classifiers (content analysis, WHOIS data, certificate analysis, link graph analysis) to score the likelihood that a reported domain is actually gambling-related.
  - **Rate limiting:** Limit the number of federated reports per agent per time period.
  - **Never auto-promote to blocklist:** All federated reports go through a review queue. No automatic blocklist additions from federated data without human approval (Phase 1-2) or high-confidence automated classification (Phase 4 ML).
  - **Abuse detection:** Monitor for coordinated reporting patterns (many reports for the same non-gambling domain from new accounts).
- **Residual Risk:** Low. The human review requirement prevents automated poisoning. Queue flooding is mitigated by automated pre-classification and rate limiting.
- **Phase:** Phase 2 (federated reporting launches in Phase 2)

#### T-009: Time Manipulation to Accelerate Unenrollment

- **Category:** Tampering
- **Description:** Self-enrolled user changes the system clock forward to make the time-delay unenrollment period appear to have elapsed.
- **Threat Actor:** TA-1
- **Attack Surface:** System clock, agent time handling
- **Likelihood:** Medium -- changing the system clock is straightforward on most OSes.
- **Impact:** High -- bypasses the time-delay unenrollment, which is the core protection for self-enrolled users.
- **Risk Rating:** High
- **Mitigation:**
  - **Server-authoritative time:** The unenrollment countdown is tracked server-side, not client-side. The agent requests unenrollment, the API starts the timer, and the API authorizes the unenrollment only after the server's clock indicates the period has elapsed.
  - **Clock drift detection:** Agent periodically compares local time to server time. Significant clock changes (forward jumps) trigger an alert and are logged.
  - **Unenrollment ceremony:** Even after the time delay, the unenrollment process requires the user to re-authenticate and explicitly confirm on the web platform. The agent cannot self-unenroll based on local time alone.
- **Residual Risk:** Very low. Server-authoritative time eliminates this attack vector entirely. The only residual risk is if the user also compromises the API, which is a different threat.
- **Phase:** Phase 1

### 4.3 Repudiation

#### T-010: Tamper Event Suppression

- **Category:** Repudiation
- **Description:** User performs a bypass or tamper action and then suppresses the evidence -- deletes logs, blocks the tamper alert from reaching the API, or modifies the event report before it is sent.
- **Threat Actor:** TA-2, TA-3
- **Attack Surface:** Agent event reporting, agent-to-API communication
- **Likelihood:** Medium
- **Impact:** High -- partner or authority does not learn about bypass attempts, undermining the accountability model.
- **Risk Rating:** High
- **Mitigation:**
  - **Event queue with local persistence:** Events are written to an encrypted, append-only local queue before transmission. The agent cannot delete events from the queue; only the API can acknowledge and remove them after successful receipt.
  - **Watchdog-generated events:** Tamper events are generated by the watchdog process, not the primary agent. Even if the primary agent is compromised, the watchdog independently reports.
  - **Heartbeat includes event digest:** Every heartbeat includes a cryptographic digest of pending events. The API can detect if events are missing (digest mismatch) or if the heartbeat itself is forged.
  - **Missed heartbeat alerting:** If the agent is taken offline to prevent reporting, the missed heartbeat triggers a partner/authority alert from the server side.
  - **Immutable server-side audit log:** All events are stored in an append-only audit log on the server. No API endpoint exists to delete or modify event records. Authority-tier logs are additionally written to a separate, tamper-evident store (future: blockchain-anchored timestamping or third-party audit service).
- **Residual Risk:** Low. The combination of watchdog-generated events, heartbeat digests, and missed heartbeat detection makes suppression very difficult. Residual risk exists if the user can keep the device offline indefinitely (see T-017).
- **Phase:** Phase 1 (event queue, heartbeat, missed heartbeat alerting), Phase 2 (watchdog events, event digests), Phase 3 (tamper-evident audit log)

#### T-011: Non-Repudiation of Partner Actions

- **Category:** Repudiation
- **Description:** A partner denies approving or denying an unenrollment request, or a partner claims they never enrolled a device (in a dispute or abuse scenario).
- **Threat Actor:** TA-5
- **Attack Surface:** Web platform, partner dashboard
- **Likelihood:** Low
- **Impact:** Medium -- disputes between users and partners, potential legal implications for authority tier.
- **Risk Rating:** Medium
- **Mitigation:**
  - **Signed action log:** All partner actions (enrollment, unenrollment approval/denial, configuration changes) are logged with timestamp, IP address, session identifier, and user agent.
  - **Email/push confirmation:** All significant partner actions trigger confirmation notifications to both the partner and the enrolled user.
  - **Action requires re-authentication:** Critical partner actions (unenrollment approval, tier changes) require re-authentication, even within an active session.
  - **Authority tier: full audit trail.** Every action is logged with sufficient detail for court proceedings.
- **Residual Risk:** Low.
- **Phase:** Phase 1 (action logging, notifications), Phase 3 (authority-tier audit trail)

### 4.4 Information Disclosure

#### T-012: Browsing History Inference from Blocked-Attempt Logs

- **Category:** Information Disclosure
- **Description:** Even though BetBlocker does not collect full browsing history, blocked-attempt logs reveal which gambling sites the user tried to visit, when, and how often. This data can be used for surveillance purposes by a malicious partner or by the platform itself.
- **Threat Actor:** TA-5, TA-6
- **Attack Surface:** Event reporting, partner dashboard, TimescaleDB
- **Likelihood:** High -- this data is collected by design for the accountability model.
- **Impact:** Medium -- temporal patterns of blocked attempts reveal behavioral information (e.g., late-night usage patterns, frequency of urges).
- **Risk Rating:** Medium
- **Mitigation:**
  - **Privacy-by-default reporting:** Self-enrolled users choose their own reporting level, including the option to disable all reporting.
  - **Aggregated reporting for partner tier:** By default, partners see aggregate statistics (e.g., "12 blocked attempts this week") not individual blocked domains. Detailed reporting requires explicit, revocable consent from the enrolled user.
  - **Data minimization:** Blocked-attempt logs store the minimum necessary data. No request content, no URL paths, no POST data -- only the domain, a category, and a timestamp.
  - **Retention limits:** Blocked-attempt data is automatically purged after a configurable retention period (default: 90 days for self-enrolled, 1 year for partner, per-policy for authority).
  - **No real-time reporting to partners:** Partners see reports on a delay (e.g., daily digest) not in real time. This prevents partners from using BetBlocker as a real-time location/activity tracker.
  - **See Privacy Threat Model (Section 5) for full analysis.**
- **Residual Risk:** Medium. The fundamental tension remains -- the accountability model requires some data sharing with partners, and any data sharing can be abused. The mitigations limit the abuse surface but cannot eliminate it entirely.
- **Phase:** Phase 1

#### T-013: API Error Message Information Leakage

- **Category:** Information Disclosure
- **Description:** API error messages reveal internal implementation details (stack traces, database schema, library versions) that help an attacker craft further attacks.
- **Threat Actor:** TA-4
- **Attack Surface:** Central API
- **Likelihood:** Medium -- a common vulnerability in web applications.
- **Impact:** Low -- information disclosure enables further attacks but is not directly harmful.
- **Risk Rating:** Low
- **Mitigation:**
  - **Generic error responses:** All API errors return standardized error codes and messages. No stack traces, no database errors, no internal paths in production.
  - **Structured error logging:** Detailed error information is logged server-side with a correlation ID. The client receives only the correlation ID for support purposes.
  - **Security headers:** Standard security headers (X-Content-Type-Options, X-Frame-Options, etc.) on all responses.
  - **Version disclosure:** Remove server version headers (X-Powered-By, Server, etc.).
- **Residual Risk:** Very low.
- **Phase:** Phase 1

#### T-014: Enrollment Relationship Disclosure

- **Category:** Information Disclosure
- **Description:** An attacker determines whether a specific person is enrolled in BetBlocker, who their partner is, or what their enrollment tier is. This is sensitive information -- it reveals that the person has a gambling problem.
- **Threat Actor:** TA-4, TA-5
- **Attack Surface:** API, web platform
- **Likelihood:** Medium
- **Impact:** High -- disclosure of gambling addiction status is deeply personal and could have professional or social consequences.
- **Risk Rating:** High
- **Mitigation:**
  - **No public profiles:** There is no public-facing information about any user. No username enumeration via login ("invalid credentials" not "user not found"). No public partner lookup.
  - **Rate-limited registration/login:** Prevent email enumeration via timing attacks on the registration and login flows. Use constant-time responses regardless of whether the email exists.
  - **Invitation privacy:** Partner invitations do not reveal the inviter's identity until the invitee accepts and authenticates.
  - **API access controls:** All API endpoints require authentication. No unauthenticated access to any user, enrollment, or device data.
  - **Database encryption:** Encryption at rest for all databases. Field-level encryption for the most sensitive fields (enrollment relationships, partner identities).
- **Residual Risk:** Low. The primary remaining risk is side-channel attacks (e.g., observing BetBlocker DNS traffic on a shared network to infer enrollment).
- **Phase:** Phase 1

#### T-015: Self-Hosted Operator Data Access

- **Category:** Information Disclosure
- **Description:** A self-hosted operator has full access to the database and can read all user data, enrollment relationships, blocked-attempt logs, and account credentials.
- **Threat Actor:** TA-6
- **Attack Surface:** PostgreSQL, TimescaleDB, Redis (on self-hosted instance)
- **Likelihood:** High -- this is inherent in the self-hosted model. The operator has root access to the server.
- **Impact:** High -- complete exposure of all user data for that instance.
- **Risk Rating:** High
- **Mitigation:**
  - **Transparent disclosure:** Documentation clearly states that self-hosted operators have full database access. Users enrolling on a self-hosted instance are explicitly warned that their data is controlled by the operator, not by BetBlocker.
  - **Password hashing:** Passwords are hashed with Argon2id (memory-hard, resistant to GPU cracking). Even the operator cannot recover plaintext passwords.
  - **End-to-end encryption for events (future):** Investigate feasibility of encrypting blocked-attempt events client-side with a key held only by the enrolled user (or partner). The server stores ciphertext. This would prevent the operator from reading event data but requires careful key management.
  - **Operator trust model documentation:** Provide clear guidance on evaluating whether to trust a self-hosted operator. Recommend that users only enroll on instances operated by organizations they trust (therapy practices, court programs).
  - **Agent-side operator validation:** The agent displays the identity of the server it is enrolled with. Users can verify they are connected to the expected instance.
- **Residual Risk:** High. This is a fundamental limitation of the self-hosted model. The operator is trusted by design. The mitigations reduce the risk of accidental or passive data exposure but cannot prevent a deliberately malicious operator from accessing the data.
- **Phase:** Phase 1 (password hashing, disclosure), Phase 3 (end-to-end encryption investigation)

### 4.5 Denial of Service

#### T-016: Agent Process Termination

- **Category:** Denial of Service
- **Description:** User kills the agent process using Task Manager, Activity Monitor, `kill -9`, or a system utility. This disables all blocking.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Agent process
- **Likelihood:** High -- this is the first thing most users will try.
- **Impact:** Critical -- complete loss of blocking for as long as the agent is down.
- **Risk Rating:** Critical
- **Mitigation:**
  - **System service model:** Agent runs as a system service (Windows Service, launchd daemon, systemd service, Android Foreground Service, iOS Network Extension). Unprivileged users cannot stop system services.
  - **Automatic restart:** OS-level service restart on crash (Windows: SCM recovery, macOS: launchd KeepAlive, Linux: systemd Restart=always, Android: START_STICKY).
  - **Watchdog process:** Separate watchdog monitors the primary agent. If the primary agent is killed, the watchdog restarts it immediately and sends a tamper alert. The watchdog is also a system service with the same protections.
  - **Mutual supervision:** The primary agent monitors the watchdog. If the watchdog is killed, the primary agent restarts it. Both must be killed simultaneously to avoid restart.
  - **Persistent network rules:** On Windows, WFP callout driver rules persist even if the agent process is killed. On Linux, iptables rules persist. The network filter blocks gambling traffic at the OS kernel level independently of the agent process.
  - **Admin access limitation:** On partner and authority tiers, recommend that the partner hold the admin credentials to the device, or that Device Admin/MDM enrollment is used to prevent admin access by the enrolled user.
- **Residual Risk:** Medium. A user with admin/root access can disable both the agent and the watchdog, and remove the persistent network rules. The mitigations make this significantly harder but not impossible. The missed heartbeat detection (T-010) provides a server-side fallback.
- **Phase:** Phase 1 (service model, restart, watchdog), Phase 2 (persistent network rules, mutual supervision)

#### T-017: Device Offline / Network Isolation

- **Category:** Denial of Service
- **Description:** User takes the device offline (airplane mode, disconnect from network) to prevent the agent from syncing blocklists, reporting events, or receiving heartbeat challenges. While offline, the user may attempt to bypass blocking using cached blocklist weaknesses or by accessing gambling through a separate network path (tethering to another device).
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Network path, agent-to-API communication
- **Likelihood:** Medium
- **Impact:** Medium -- blocking continues offline (cached blocklist), but reporting and accountability are disrupted.
- **Risk Rating:** Medium
- **Mitigation:**
  - **Offline blocking:** The agent maintains a full local blocklist cache. Blocking continues even with no network connectivity. This is a core design principle.
  - **Missed heartbeat alerting:** Server-side monitoring detects when a device misses its heartbeat window. Partner/authority is alerted (email/push) that the device has gone offline.
  - **Event queuing:** All events (blocks, tamper detections) are queued locally and transmitted when connectivity is restored. Events include the original timestamp, not the transmission timestamp.
  - **Network change monitoring:** Agent monitors for network interface changes. If the device connects to a new network (e.g., mobile tether) while the primary interface is down, the agent activates on the new interface.
  - **Configurable offline tolerance:** Authority tier can configure a maximum offline duration after which the agent escalates (e.g., enters a lockdown mode where the device's network access is restricted to essential services only until connectivity with the API is restored).
- **Residual Risk:** Medium. A user can keep the device offline indefinitely. The missed heartbeat alert is the primary mitigation, shifting responsibility to the partner/authority to follow up.
- **Phase:** Phase 1 (offline blocking, heartbeat, event queuing), Phase 2 (network change monitoring), Phase 3 (configurable offline lockdown)

#### T-018: DDoS Against Central API

- **Category:** Denial of Service
- **Description:** Volumetric or application-layer DDoS attack against the central API. If the API is unavailable, agents cannot sync blocklists, report events, or process unenrollment requests.
- **Threat Actor:** TA-4
- **Attack Surface:** Central API, web platform
- **Likelihood:** Medium
- **Impact:** Medium -- agents continue blocking offline (T-017 mitigations apply). The primary impact is that new enrollments, unenrollment requests, and partner actions are disrupted.
- **Risk Rating:** Medium
- **Mitigation:**
  - **CDN/WAF:** CloudFront or Cloudflare WAF in front of all public endpoints. DDoS protection at the edge.
  - **Rate limiting:** Per-IP, per-account, and per-endpoint rate limits. Separate rate limit tiers for agent-to-API traffic (higher limits) and public web traffic.
  - **Horizontal scaling:** Stateless API servers behind a load balancer. Autoscaling based on traffic.
  - **Agent resilience:** Agents gracefully handle API unavailability. Blocking continues. Events queue. Exponential backoff on retries.
  - **Critical path isolation:** The blocklist sync and heartbeat endpoints are on a separate service/path from the web platform, so a DDoS on the marketing site does not affect enrolled devices.
- **Residual Risk:** Low for blocking functionality (offline resilience). Medium for platform availability.
- **Phase:** Phase 1 (rate limiting, agent resilience), Phase 2 (CDN/WAF, autoscaling, path isolation)

#### T-019: OS Reinstallation / Factory Reset

- **Category:** Denial of Service
- **Description:** User reinstalls the operating system or performs a factory reset, wiping the agent entirely.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Operating system
- **Likelihood:** Medium -- drastic but effective. Higher for mobile (factory reset is easy) than desktop (OS reinstall is disruptive).
- **Impact:** Critical -- complete removal of all blocking.
- **Risk Rating:** Critical
- **Mitigation:**
  - **MDM enrollment (iOS/macOS):** MDM profiles survive most reset scenarios. iOS Supervised mode prevents factory reset without MDM authority.
  - **Device Administrator/Owner (Android):** Android Device Owner enrollment survives factory reset on some devices (Samsung Knox). Standard Device Admin can be removed before reset on most devices.
  - **BIOS/UEFI protection (future, enterprise):** For institutional deployments, investigate BIOS-level persistence. This is extreme and only appropriate for authority-tier with explicit consent.
  - **Detection and alerting:** The API detects that a previously enrolled device has gone silent (missed heartbeats). Partner/authority is alerted. The alert message explicitly warns that the device may have been reset.
  - **Re-enrollment friction:** For partner and authority tiers, the partner/authority is notified and can require re-enrollment as a condition of the accountability agreement. The technical mitigation is limited; the social/legal mitigation is primary.
  - **Enrollment binding:** For Android with Samsung Knox and iOS with Supervision, the device cannot be factory reset without the enrollment authority's approval. This is the strongest mitigation but requires specific hardware and enrollment at the device setup level.
- **Residual Risk:** High on platforms without MDM/Knox. Medium on iOS with Supervision and Android with Knox. The server-side detection ensures the partner/authority is informed, but cannot prevent the reset itself.
- **Phase:** Phase 1 (detection, alerting), Phase 2 (Device Admin), Phase 3 (MDM, Knox, supervised enrollment)

### 4.6 Elevation of Privilege

#### T-020: Privilege Escalation to Disable Agent

- **Category:** Elevation of Privilege
- **Description:** User escalates from standard user to administrator/root to gain the privileges necessary to kill the agent, modify its files, or disable OS-level protections.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Operating system privilege model
- **Likelihood:** High -- many users are already administrators of their own devices. Those who are not can often escalate (password recovery, boot from USB, etc.).
- **Impact:** Critical -- admin access is a prerequisite for most bypass techniques.
- **Risk Rating:** Critical
- **Mitigation:**
  - **Assume admin access:** The threat model assumes the user has or can obtain admin access. All agent protections must be designed to resist admin-level attacks, not just standard-user attacks.
  - **Kernel-level protections:** WFP callout drivers, macOS System Extensions, and Linux MAC policies operate at the kernel level. Even an administrator cannot trivially disable them without rebooting (and the agent can detect and respond to reboots).
  - **Secure boot integration (future):** On platforms with Secure Boot, investigate agent integration with the boot chain. Agent protections load before user-mode admin tools.
  - **Separate admin account (recommendation):** For partner and authority tiers, recommend that the enrolled user's account is a standard user account, with admin credentials held by the partner or IT administrator.
  - **UAC/sudo monitoring:** Agent monitors for privilege escalation events (UAC prompts, sudo usage) and logs them. For authority tier, these can be reported.
- **Residual Risk:** High. This is the fundamental challenge of BetBlocker. The device owner has physical access and can, with sufficient effort, gain kernel-level access on most consumer platforms. The mitigations raise the bar significantly but cannot make bypass impossible. The key insight is: the goal is to make bypass require enough time, effort, and deliberate action that the urge passes.
- **Phase:** Phase 1 (assume admin, defense in depth), Phase 2 (kernel-level protections), Phase 3 (secure boot investigation)

#### T-021: Enrollment Tier Escalation (API)

- **Category:** Elevation of Privilege
- **Description:** A standard user accesses partner or authority API endpoints. A partner-enrolled user manipulates the API to self-approve their own unenrollment. An enrolled user modifies their own enrollment tier to reduce protection level.
- **Threat Actor:** TA-1, TA-2, TA-3, TA-4
- **Attack Surface:** Central API, enrollment endpoints
- **Likelihood:** Medium
- **Impact:** Critical -- bypasses the entire enrollment authority model.
- **Risk Rating:** Critical
- **Mitigation:**
  - **Role-based access control (RBAC):** Strict role separation: user, partner, authority, admin. Each role can only access endpoints appropriate to their role.
  - **Enrollment-aware authorization:** API authorization checks not just the role but the specific enrollment relationship. A user can only modify their own enrollments. A partner can only manage enrollments they created. An authority can only manage enrollments under their organization.
  - **Unenrollment policy enforcement:** The unenrollment endpoint enforces the enrollment's unenrollment policy. Self-enrolled: time delay. Partner-enrolled: partner approval required (separate authenticated session from the partner's account). Authority: authority approval required.
  - **Cross-account isolation:** A user and their partner must have separate accounts with separate credentials. The API rejects any attempt to perform partner actions from the enrolled user's session.
  - **Audit logging:** All enrollment changes are logged with the acting account, IP, timestamp, and the enrollment's policy at the time of the action.
  - **Input validation:** API rejects any request that attempts to set a tier or protection level that is lower than what the current enrollment authority permits.
- **Residual Risk:** Low, assuming the RBAC implementation is correct. Authorization bugs are a common vulnerability class, so this requires thorough testing (see T-033).
- **Phase:** Phase 1

#### T-022: Boot into Safe Mode / Recovery Mode

- **Category:** Elevation of Privilege
- **Description:** User boots the device into safe mode, recovery mode, or from an external boot device (USB) to gain access to the filesystem without the agent running. In safe mode, third-party services and drivers do not load.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Operating system boot process
- **Likelihood:** Medium -- safe mode is well-documented. Recovery mode and USB boot require slightly more sophistication.
- **Impact:** High -- allows file modification, agent removal, and configuration tampering without the agent running to detect or prevent it.
- **Risk Rating:** High
- **Mitigation:**
  - **Safe mode detection:** On Windows, the agent checks the boot mode on startup. If booted in safe mode, log the event, restrict network access (safe mode with networking), and alert the API on first connectivity.
  - **BIOS password recommendation:** For partner and authority tiers, recommend setting a BIOS/firmware password to prevent boot from external devices. Partner/authority holds the password.
  - **BitLocker/FileVault/LUKS:** Full-disk encryption prevents access from external boot devices without the encryption password.
  - **Persistent driver (Windows):** WFP callout drivers can be configured to load even in safe mode (boot-start drivers). This provides blocking even in safe mode with networking.
  - **Post-boot integrity check:** On every normal boot, the agent performs a full integrity check of all its files and configuration. Any changes made during safe mode/recovery are detected and trigger a tamper alert plus self-healing.
  - **Heartbeat gap detection:** The server detects the heartbeat gap during the safe mode session and alerts the partner/authority.
- **Residual Risk:** Medium. Safe mode with networking can be mitigated with persistent drivers. Safe mode without networking or USB boot cannot be technically prevented on most consumer hardware, only detected after the fact.
- **Phase:** Phase 1 (post-boot integrity, heartbeat gap), Phase 2 (safe mode detection, persistent driver), Phase 3 (BIOS recommendations, FDE integration)

### 4.7 Additional Threats (Beyond STRIDE)

#### T-023: VPN / Proxy / Tor Bypass

- **Category:** Tampering (network path)
- **Description:** User installs a VPN, configures a proxy, or uses Tor to tunnel traffic through an encrypted channel that bypasses the agent's DNS interception and network filtering.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Network path
- **Likelihood:** High -- VPN apps are readily available and commonly known as DNS filter bypasses.
- **Impact:** Critical -- complete bypass of DNS/network-level blocking. VPN encrypts all traffic, so the agent cannot inspect it.
- **Risk Rating:** Critical
- **Mitigation:**
  - **VPN detection:** Agent monitors for new VPN connections (tun/tap interfaces, WireGuard, OpenVPN, IPSec). On detection, alert and optionally block.
  - **Proxy detection:** Monitor system proxy settings, PAC files, and common proxy ports.
  - **Tor detection:** Block known Tor entry nodes. Detect the Tor browser binary and process.
  - **Application blocking (Layer 2):** Block installation and execution of known VPN and proxy applications that could be used for bypass.
  - **Platform network hooks as VPN:** On Android, the agent uses VpnService (local VPN). Only one VPN can be active at a time. Installing a second VPN disconnects the agent's VPN, which triggers an immediate tamper alert. On iOS, NetworkExtension similarly restricts concurrent configurations.
  - **Network-level enforcement:** On Windows (WFP) and Linux (iptables), the agent can block traffic to non-whitelisted endpoints that do not pass through the agent's filter. This is the nuclear option and must be carefully scoped to avoid breaking legitimate traffic.
  - **Response tiers:** Self-enrolled: log and optionally alert. Partner-enrolled: alert partner. Authority: alert authority and optionally restrict network to known-safe destinations.
- **Residual Risk:** Medium. Novel VPN protocols, SSH tunnels, and obfuscated proxies can evade detection. The application blocking layer (Phase 2) is the strongest mitigation -- prevent the VPN app from running in the first place.
- **Phase:** Phase 1 (VPN/proxy/Tor detection, alerting), Phase 2 (application blocking of VPN apps, network enforcement)

#### T-024: Use of a Separate Device

- **Category:** Out of band (not a software threat)
- **Description:** User simply uses a different device (friend's phone, library computer, gaming console) that does not have BetBlocker installed.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Out of scope for the endpoint agent, but in scope for the product.
- **Likelihood:** Very high -- this is the easiest "bypass" and requires no technical skill.
- **Impact:** Critical -- the user accesses gambling with no blocking or detection.
- **Risk Rating:** Critical (by impact), but outside the technical scope of the agent.
- **Mitigation:**
  - **Multi-device enrollment:** Encourage and facilitate enrollment of all the user's devices. Make the enrollment process as frictionless as possible.
  - **Partner/authority awareness:** Educate partners and authorities that BetBlocker only protects enrolled devices. The accountability relationship extends beyond the software.
  - **Network-level blocking (future):** For home networks, a BetBlocker-configured router or Pi-hole style appliance could block gambling traffic for all devices on the network, including those without the agent.
  - **This is fundamentally a social/behavioral problem, not a technical one.** The threat model acknowledges it but does not claim to solve it.
- **Residual Risk:** High. No software agent can prevent the use of an unenrolled device.
- **Phase:** Phase 1 (multi-device UX), Phase 4 (network appliance)

#### T-025: Reverse Engineering the Agent

- **Category:** Information Disclosure + Elevation of Privilege
- **Description:** Attacker reverse-engineers the agent binary to understand its protection mechanisms, find vulnerabilities, or extract embedded secrets (API keys, certificate pins).
- **Threat Actor:** TA-2, TA-3 (personal bypass), TA-4 (platform attack), TA-7 (supply chain intelligence)
- **Attack Surface:** Agent binary
- **Likelihood:** Medium -- Rust binaries are harder to reverse than managed languages but not immune.
- **Impact:** High -- understanding of protection mechanisms enables targeted bypasses. Extraction of embedded secrets enables API spoofing.
- **Risk Rating:** High
- **Mitigation:**
  - **No security through obscurity:** Do not rely on the agent's internal logic being secret. All protection mechanisms must be secure even if the attacker has full source code (Kerckhoffs' principle). Open-source release (Phase 4) makes this explicit.
  - **No embedded secrets:** The agent binary does not contain API keys, master secrets, or other high-value credentials. Device-specific credentials are generated at enrollment time and stored in hardware-backed key stores.
  - **Certificate pinning via configuration:** Certificate pins are in the signed configuration, not hardcoded in the binary. This allows rotation without binary updates.
  - **Binary hardening:** Strip debug symbols, enable Rust's release optimizations, and consider code obfuscation for particularly sensitive logic (tamper detection, watchdog communication). Note: obfuscation is defense-in-depth, not a primary control.
  - **Anti-debugging:** Detect if a debugger is attached and alert. Do not rely on this as a primary defense (it can be bypassed), but it raises the bar.
- **Residual Risk:** Medium. The security model does not depend on binary secrecy, so reverse engineering reveals implementation details but does not inherently enable bypass. The main risk is discovery of implementation bugs.
- **Phase:** Phase 1 (no embedded secrets, Kerckhoffs' principle), Phase 2 (binary hardening, anti-debugging)

#### T-026: Agent Update Hijacking

- **Category:** Tampering + Elevation of Privilege
- **Description:** Attacker compromises the update mechanism to deliver a malicious agent binary. Since the agent runs as SYSTEM/root, this gives the attacker full control of every enrolled device.
- **Threat Actor:** TA-7 (supply chain), TA-4 (infrastructure compromise)
- **Attack Surface:** Update mechanism, CDN, signing infrastructure
- **Likelihood:** Low (requires significant compromise of build infrastructure)
- **Impact:** Critical -- SYSTEM/root access on all enrolled devices globally.
- **Risk Rating:** Critical
- **Mitigation:**
  - **Code signing:** All agent binaries are signed with a BetBlocker code signing key. Agent validates the signature before applying an update. Signature verification uses a hardcoded public key embedded in the agent (only the public key, not the signing key).
  - **Dual signing:** Updates are signed by both an automated build key and a release key held offline. Both signatures must validate. Compromise of the automated build system alone is insufficient.
  - **Transparency log:** All released binaries and their signatures are published to a transparency log (sigstore/Rekor or a self-hosted equivalent). Any unauthorized release is detectable.
  - **Staged rollout:** Updates roll out gradually (1% -> 10% -> 50% -> 100%) with monitoring for anomalies (crash rates, tamper alerts, heartbeat disruptions) at each stage. Rollback is automatic if anomaly thresholds are exceeded.
  - **Reproducible builds:** The build process is deterministic. Anyone can build from source and verify that the resulting binary matches the signed release. Open-source (Phase 4) makes this universally verifiable.
  - **Update pinning:** The agent only accepts updates with a version higher than its current version. Rollbacks require explicit authorization from the API.
  - **Self-hosted operator builds:** Self-hosted operators who build from source sign with their own key. Their agents validate against the operator's key, not BetBlocker's. This isolates the supply chains.
- **Residual Risk:** Very low, given dual signing, transparency logging, and staged rollout. The remaining risk is compromise of the offline release key, which should be stored on hardware security modules (HSM) with access controls and audit logging.
- **Phase:** Phase 1 (code signing, version pinning), Phase 2 (dual signing, staged rollout), Phase 3 (transparency log, reproducible builds)

#### T-027: Tethering / Hotspot Bypass

- **Category:** Tampering (network path)
- **Description:** User connects the enrolled device to the internet through a second device's hotspot (e.g., phone tethering to laptop), where the second device's network path does not go through the agent's filter.
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Network path
- **Likelihood:** Medium
- **Impact:** Low to none -- the agent's network hooks (WFP, NetworkExtension, VpnService, iptables) intercept traffic on the enrolled device regardless of the network path. DNS interception happens on the device, not on the network. Changing the upstream network does not bypass the agent.
- **Risk Rating:** Low
- **Mitigation:**
  - **No additional mitigation needed.** The agent's platform network hooks already handle this. DNS interception and network filtering operate at the device level, not the network level. The upstream network path is irrelevant.
  - **Clarification for users:** Document that BetBlocker blocks gambling on the enrolled device regardless of what network the device is connected to.
- **Residual Risk:** Very low. This is only a concern if the platform network hooks have bugs or if the user bypasses the hooks (which is covered by other threats like T-023).
- **Phase:** Phase 1 (by design)

#### T-028: Browser-Based Bypass (Incognito, Alternative Browser)

- **Category:** Tampering
- **Description:** User uses an incognito/private browsing window, a portable browser, or a lesser-known browser to bypass browser extension-based blocking (Layer 3).
- **Threat Actor:** TA-1, TA-2, TA-3
- **Attack Surface:** Browser/content blocking layer
- **Likelihood:** High for browser extension bypass. Incognito mode disables extensions by default on most browsers.
- **Impact:** Medium -- this only bypasses Layer 3 (content blocking). Layers 1 (DNS) and 2 (app blocking) are unaffected. The user can still not reach gambling domains even without the extension.
- **Risk Rating:** Medium
- **Mitigation:**
  - **Layers 1 and 2 are the primary controls.** Browser extensions (Layer 3) are defense-in-depth for content on non-blocked domains (e.g., gambling ads on sports sites). DNS and network blocking are not browser-dependent.
  - **Extension presence monitoring:** The agent monitors whether the browser extension is installed and active. If the extension is disabled or removed, the agent alerts and logs the event.
  - **Force extension in incognito:** On Chrome and Edge, enterprise policies can force extensions to run in incognito mode. The agent configures this policy.
  - **Alternative browser detection:** Agent monitors for installation and execution of browsers that do not have the extension installed. Alert and, for Layer 2, optionally block unmanaged browsers.
- **Residual Risk:** Low. The multi-layer approach means that browser extension bypass does not provide access to gambling sites, only to gambling-related content on non-gambling sites.
- **Phase:** Phase 3 (Layer 3 is Phase 3)

#### T-029: Debugging and Memory Manipulation

- **Category:** Tampering + Elevation of Privilege
- **Description:** Attacker attaches a debugger (WinDbg, lldb, gdb) to the running agent process to inspect its memory, patch out protection checks, or modify the in-memory blocklist.
- **Threat Actor:** TA-2, TA-3
- **Attack Surface:** Agent process memory
- **Likelihood:** Low -- requires significant technical skill and admin access.
- **Impact:** High -- can disable any protection mechanism at runtime.
- **Risk Rating:** Medium
- **Mitigation:**
  - **Anti-debugging:** Agent detects if a debugger is attached (platform-specific APIs: `IsDebuggerPresent()` on Windows, `PT_DENY_ATTACH` on macOS, `/proc/self/status` on Linux). On detection, alert and terminate.
  - **Process protection:** On Windows, set the agent process as a Protected Process Light (PPL) using an Early Launch Antimalware (ELAM) driver. PPL processes cannot be debugged even by administrators.
  - **macOS hardened runtime:** Build with hardened runtime entitlements, which prevent debugging and code injection.
  - **Linux ptrace restriction:** Set `PR_SET_DUMPABLE` to 0 and leverage Yama LSM `ptrace_scope` settings.
  - **Watchdog cross-check:** Watchdog periodically validates the primary agent's memory integrity (code section hashes). Detects runtime patching.
- **Residual Risk:** Low on Windows (PPL is strong) and macOS (hardened runtime). Medium on Linux and Android (anti-debugging is more easily circumvented).
- **Phase:** Phase 2

### 4.8 Web Platform Threats (OWASP Top 10)

#### T-030: SQL Injection

- **Category:** Tampering
- **Description:** Injection of malicious SQL through API inputs that reach the database.
- **Threat Actor:** TA-4
- **Attack Surface:** Central API
- **Likelihood:** Low -- Rust's type system and use of parameterized queries (sqlx) make accidental SQL injection unlikely.
- **Impact:** Critical -- database compromise, data exfiltration, data modification.
- **Risk Rating:** Medium
- **Mitigation:**
  - **Parameterized queries only:** All database access uses sqlx compile-time checked queries. No string concatenation for SQL.
  - **Input validation:** All API inputs validated via Rust types and serde deserialization constraints before reaching any database layer.
  - **Least privilege database user:** The API connects to PostgreSQL with a role that has minimum necessary permissions (no DROP, no GRANT, no raw DDL).
  - **SAST scanning:** Semgrep rules for SQL injection patterns in CI/CD pipeline.
- **Residual Risk:** Very low.
- **Phase:** Phase 1

#### T-031: Cross-Site Scripting (XSS)

- **Category:** Tampering
- **Description:** Injection of malicious JavaScript through user-supplied content rendered in the web platform.
- **Threat Actor:** TA-4
- **Attack Surface:** Web platform (Next.js)
- **Likelihood:** Low -- React's default escaping prevents most XSS. Server-side rendering adds some surface area.
- **Impact:** High -- session hijacking, account takeover, phishing within the platform.
- **Risk Rating:** Medium
- **Mitigation:**
  - **React default escaping:** All user content rendered through React's JSX escaping. Never use `dangerouslySetInnerHTML`.
  - **Content Security Policy:** Strict CSP header that blocks inline scripts and restricts script sources to own domain. No `unsafe-inline`, no `unsafe-eval`.
  - **Input sanitization:** Server-side sanitization of all user-supplied content before storage. Use a mature sanitization library (DOMPurify equivalent for any server-side HTML rendering).
  - **HTTPOnly, Secure, SameSite cookies:** Session cookies are HTTPOnly (no JavaScript access), Secure (HTTPS only), and SameSite=Strict.
- **Residual Risk:** Very low.
- **Phase:** Phase 1

#### T-032: Cross-Site Request Forgery (CSRF)

- **Category:** Spoofing
- **Description:** Attacker tricks an authenticated user's browser into making an unwanted request to the BetBlocker API (e.g., approving an unenrollment).
- **Threat Actor:** TA-1 (craft a CSRF payload that approves their own unenrollment when the partner clicks a link), TA-4
- **Attack Surface:** Web platform, API
- **Likelihood:** Medium -- CSRF is a well-known attack with established defenses, but the partner-approval flow is a particularly attractive target.
- **Impact:** High -- unauthorized unenrollment approval, enrollment changes.
- **Risk Rating:** High
- **Mitigation:**
  - **SameSite cookies:** SameSite=Strict on all session cookies. This is the primary CSRF defense.
  - **CSRF tokens:** Double-submit cookie pattern or synchronizer token for all state-changing requests.
  - **Custom request header:** API requires a custom header (e.g., `X-BetBlocker-Request: 1`) on all state-changing requests. Browsers do not send custom headers in cross-origin requests without CORS preflight.
  - **Re-authentication for critical actions:** Unenrollment approval, partner changes, and tier modifications require re-authentication (password + MFA), not just a session cookie. This renders CSRF useless for the most sensitive actions.
  - **Origin header validation:** API validates the Origin header on all requests and rejects cross-origin state-changing requests.
- **Residual Risk:** Very low.
- **Phase:** Phase 1

#### T-033: Broken Access Control (IDOR / Privilege Escalation)

- **Category:** Elevation of Privilege
- **Description:** A user accesses another user's data or performs actions they should not be authorized to perform by manipulating object IDs, API parameters, or endpoint paths.
- **Threat Actor:** TA-1, TA-2, TA-4
- **Attack Surface:** Central API
- **Likelihood:** Medium -- authorization bugs are the #1 OWASP vulnerability class.
- **Impact:** Critical -- access to other users' data, unauthorized enrollment changes, privacy violations.
- **Risk Rating:** Critical
- **Mitigation:**
  - **Authorization middleware:** Centralized authorization middleware that validates the requesting account's relationship to the requested resource on every API call. No endpoint should implement its own ad-hoc authorization checks.
  - **Use opaque identifiers:** External-facing IDs should be UUIDs, not sequential integers. This does not prevent IDOR (security through obscurity) but makes enumeration harder.
  - **Resource ownership validation:** Every API call that accesses a resource validates that the requesting account owns or has a legitimate relationship to that resource. The authorization layer loads the resource and checks ownership before the handler executes.
  - **Automated authorization testing:** Integration tests that systematically test every endpoint with every role (user accessing partner endpoints, user A accessing user B's data, etc.). These tests are part of CI/CD and block releases.
  - **Security review:** All new endpoints undergo security review with a focus on authorization before merge.
- **Residual Risk:** Low, with disciplined authorization middleware and comprehensive testing. Medium without.
- **Phase:** Phase 1

#### T-034: Server-Side Request Forgery (SSRF)

- **Category:** Tampering
- **Description:** Attacker tricks the API server into making requests to internal services or arbitrary URLs, potentially accessing internal network resources.
- **Threat Actor:** TA-4
- **Attack Surface:** Central API (any endpoint that accepts URLs or makes outbound requests based on user input)
- **Likelihood:** Low -- the API has limited URL-fetching functionality. The automated discovery pipeline (Phase 2+) is the primary risk area.
- **Impact:** High -- access to internal services, cloud metadata endpoints, private networks.
- **Risk Rating:** Medium
- **Mitigation:**
  - **URL allowlisting:** Any endpoint that fetches URLs (e.g., blocklist source verification, automated discovery) validates the URL against an allowlist of permitted schemes (https only) and blocks private IP ranges (RFC 1918, link-local, loopback, cloud metadata IPs like 169.254.169.254).
  - **DNS re-resolution protection:** After URL validation, re-resolve the hostname and re-check the IP. Prevent DNS rebinding attacks where the hostname resolves to a public IP during validation but a private IP during the actual request.
  - **Network segmentation:** The API server's outbound network access is restricted by security groups/firewall rules to only the destinations it legitimately needs (databases, cache, external APIs).
  - **Separate service for URL fetching:** The automated discovery pipeline runs on a separate, isolated service with no access to internal databases or services. It can only write to a message queue.
- **Residual Risk:** Very low.
- **Phase:** Phase 1 (URL validation), Phase 2 (isolated discovery service)

#### T-035: Session Hijacking

- **Category:** Spoofing
- **Description:** Attacker steals a user's session token to impersonate them on the web platform.
- **Threat Actor:** TA-1 (steal partner's session), TA-4
- **Attack Surface:** Web platform, API
- **Likelihood:** Medium
- **Impact:** High -- full access to the hijacked account, including partner actions.
- **Risk Rating:** High
- **Mitigation:**
  - **Short-lived access tokens:** JWT access tokens with 15-minute expiration. Refresh tokens with longer expiration stored in HTTPOnly cookies.
  - **Refresh token rotation:** Each refresh token can only be used once. Using a refresh token invalidates the previous one. If a stolen refresh token is used after the legitimate user has rotated it, all sessions for that account are invalidated (theft detection).
  - **Session binding:** Sessions are bound to a fingerprint of the client (IP range, user-agent). Significant changes trigger re-authentication.
  - **Secure cookie attributes:** HTTPOnly, Secure, SameSite=Strict, Path restricted.
  - **TLS everywhere:** HTTPS only, with HSTS. No HTTP endpoints.
  - **Logout invalidation:** Logout invalidates the refresh token server-side. Not just a client-side cookie deletion.
- **Residual Risk:** Low.
- **Phase:** Phase 1

---

## 5. Privacy Threat Model

### 5.1 Fundamental Tension

BetBlocker exists in a privacy paradox:

- **For self-enrolled users:** The user wants blocking but may not want anyone to know they have a gambling problem. Maximum privacy.
- **For partner-enrolled users:** The partner needs visibility into the enrolled user's compliance. This necessarily involves some data sharing about gambling-related activity on the enrolled user's device.
- **For authority-enrolled users:** The court or institution requires verifiable evidence of compliance. This requires the most data collection and disclosure.

The platform must support all three models without any tier's data requirements contaminating the others.

### 5.2 Data Inventory

| Data Type | Collected | Stored Server-Side | Shared With Partner | Shared With Authority | Retention |
|-----------|----------|-------------------|--------------------|-----------------------|-----------|
| Account credentials (hashed password) | Yes | Yes (Argon2id hash) | Never | Never | Account lifetime |
| Email address | Yes | Yes | Only for invitation | Organization admin | Account lifetime |
| Device hardware identifiers | Yes (enrollment) | Yes (for device binding) | Never (only device name) | Device ID only | Enrollment lifetime |
| Blocked-attempt domains | Yes | Yes | Aggregate by default; detailed with consent | Full detail | Configurable (90d default) |
| Blocked-attempt timestamps | Yes | Yes | Aggregate by default | Full detail | Configurable (90d default) |
| Browsing history (non-blocked) | **Never** | **Never** | N/A | N/A | N/A |
| Installed applications | Phase 2 (matched only) | Only matched gambling apps | App names with consent | Full detail | Enrollment lifetime |
| VPN/proxy usage | Phase 2 (detection only) | Detection events only | Alert only | Full detail | Configurable |
| Device location | **Never** | **Never** | N/A | N/A | N/A |
| Keystrokes | **Never** | **Never** | N/A | N/A | N/A |
| Screen content | **Never** | **Never** | N/A | N/A | N/A |
| Microphone/camera | **Never** | **Never** | N/A | N/A | N/A |

### 5.3 Privacy Threat: Surveillance Abuse by Partner (PT-001)

- **Description:** A controlling partner uses BetBlocker to monitor and control their victim. Even with BetBlocker's limited data collection, the following can be abused:
  - Device online/offline status (is the person at home? is their phone on?)
  - Blocked-attempt timing patterns (when is the person most vulnerable?)
  - Tamper alert notifications (is the person trying to remove monitoring?)
  - Unenrollment denial (preventing the person from removing blocking they no longer want)
- **Mitigations:**
  - **Self-unenrollment escape hatch:** Even for partner-enrolled devices, the enrolled user can always initiate a "break glass" unenrollment that removes blocking immediately but permanently notifies the partner that the unenrollment was forced. This prevents the platform from being used as a tool of coercive control. The partner cannot block this action, only be informed of it.
  - **No real-time location or activity data:** BetBlocker does not collect location data, and blocked-attempt reports are delivered on a delay (daily digest), not in real time.
  - **Minimal data by default:** Partner reporting defaults to aggregated data. The enrolled user must actively consent to detailed reporting and can revoke that consent at any time.
  - **Abuse reporting:** Provide an in-app mechanism for enrolled users to report partner abuse. BetBlocker support investigates and can sever the partner relationship.
  - **Clear documentation:** Onboarding flow for partner enrollment explicitly states what data the partner will see and what they will not see. Both parties acknowledge the data sharing agreement.
- **Residual Risk:** Medium. The platform minimizes the abuse surface, but any tool that gives one person visibility into another's device can be misused. The "break glass" mechanism ensures the enrolled user always has an escape.

### 5.4 Privacy Threat: Platform as Spyware (PT-002)

- **Description:** BetBlocker itself could become spyware if the codebase is modified (by a rogue developer, a compromised build, or a rogue self-hosted operator) to collect more data than disclosed.
- **Mitigations:**
  - **Open-source code (Phase 4):** The agent source code is publicly auditable. Anyone can verify exactly what data the agent collects.
  - **Reproducible builds:** Users can build the agent from source and verify it matches the official binary. Modified builds are detectable.
  - **Privacy-preserving architecture:** The agent collects the minimum data needed for its function. The architecture makes it structurally difficult to expand data collection without visible code changes (e.g., adding a new event type requires changes in the event schema, the reporting module, and the API ingestion -- all reviewable).
  - **No raw browsing data pathway:** The agent does not have a mechanism to exfiltrate arbitrary data. It only sends structured events matching a defined schema. There is no generic "upload data" endpoint.
  - **Third-party audit:** Periodic third-party security and privacy audits of the codebase and deployed infrastructure. Published audit reports.
  - **Transparency report:** Regular transparency reports disclosing what data is collected, how it is used, and any law enforcement requests.
- **Residual Risk:** Low for the hosted platform (with audits). Medium for self-hosted (operator controls the server).

### 5.5 Privacy Threat: Federated Report Deanonymization (PT-003)

- **Description:** Federated reports (agents reporting unknown gambling domains back to the central pipeline) could be used to identify which user reported which domain, potentially revealing browsing patterns.
- **Mitigations:**
  - **Report stripping:** Federated reports are stripped of device identifiers, account identifiers, and timestamps before submission to the central pipeline. Reports include only the domain, a category confidence score, and a coarse geographic region (country-level, for regional blocklist maintenance).
  - **Batched submission:** Reports are batched and submitted at randomized intervals, not immediately. This prevents correlation between a specific user's browsing and a report arrival.
  - **Opt-in only:** Federated reporting is opt-in. Users must explicitly enable it. Self-hosted operators can disable it entirely.
  - **Differential privacy (future):** Investigate differential privacy mechanisms for federated reporting to provide mathematical guarantees against deanonymization.
  - **Report aggregation threshold:** A report is only forwarded to the review queue if multiple independent agents report the same domain. Individual reports are discarded. This prevents the platform from learning that a specific user visited a specific unreported domain.
- **Residual Risk:** Low with stripping and batching. Very low with aggregation thresholds.

### 5.6 Privacy Threat: Data Breach Exposure (PT-004)

- **Description:** If the BetBlocker database is breached, attackers gain access to a list of people with gambling problems, their accountability partners, and their blocked-attempt history.
- **Mitigations:**
  - **Encryption at rest:** All databases encrypted at rest (AWS KMS, GCP CMEK, or LUKS for self-hosted).
  - **Field-level encryption:** Sensitive fields (email, partner relationships) encrypted with application-level keys, so a database dump alone does not expose plaintext.
  - **Data minimization:** Store the minimum data necessary. No browsing history. Retention limits with automatic purging.
  - **Password hashing:** Argon2id with high memory cost. Breached password hashes are computationally infeasible to crack.
  - **Incident response plan:** Documented breach response plan including notification timelines (GDPR 72h), communication templates, and remediation procedures.
  - **Segmented storage:** Authentication data, enrollment data, and event analytics are stored in separate databases with separate access credentials. A breach of one does not compromise all.
- **Residual Risk:** Medium. No system is immune to breach. The mitigations ensure that a breach exposes minimal useful data.

---

## 6. Anti-Bypass Matrix

This matrix maps bypass techniques to countermeasures for each platform. Each cell indicates the countermeasure and the phase in which it is implemented.

### 6.1 Agent Process Termination

| Bypass Technique | Windows | macOS | Linux | Android | iOS |
|-----------------|---------|-------|-------|---------|-----|
| Kill via task manager / UI | Windows Service (P1) | launchd daemon (P1) | systemd service (P1) | Foreground Service (P1) | Network Extension (P1) |
| Kill via command line (kill -9, taskkill) | Service ACLs (P1) | launchd KeepAlive (P1) | systemd Restart=always (P1) | START_STICKY (P1) | N/A (sandboxed) |
| Disable service | Registry protection (P2) | SIP protects launchd config (P1) | systemd mask protection (P2) | Device Admin (P2) | MDM profile (P3) |
| Watchdog evasion | Mutual supervision (P2) | Mutual supervision (P2) | Mutual supervision (P2) | Mutual supervision (P2) | N/A (OS manages) |

### 6.2 Agent Binary/Config Modification

| Bypass Technique | Windows | macOS | Linux | Android | iOS |
|-----------------|---------|-------|-------|---------|-----|
| Replace binary | Kernel minifilter (P2) | Endpoint Security (P2) | chattr +i, SELinux (P2) | App signing (P1) | App Store signing (P1) |
| Modify config | TPM-bound encryption (P1) | Keychain encryption (P1) | Filesystem key (P1) | Keystore encryption (P1) | Keychain encryption (P1) |
| Delete agent files | Kernel minifilter (P2) | Endpoint Security (P2) | SELinux (P2) | Device Admin (P2) | MDM profile (P3) |
| Self-healing | Cached signed copy (P1) | Cached signed copy (P1) | Cached signed copy (P1) | Cached signed copy (P1) | N/A (App Store) |

### 6.3 DNS/Network Bypass

| Bypass Technique | Windows | macOS | Linux | Android | iOS |
|-----------------|---------|-------|-------|---------|-----|
| Change DNS servers | WFP intercepts all DNS (P1) | NetworkExtension (P1) | iptables redirect (P1) | VpnService captures DNS (P1) | NEDNSProxyProvider (P1) |
| Use DoH/DoT | Block known DoH endpoints via WFP (P2) | Block via NetworkExtension (P2) | Block via iptables (P2) | VpnService intercepts (P1) | NEDNSProxyProvider (P1) |
| Edit HOSTS file | Monitor + revert (P1) | Monitor + revert (P1) | Monitor + revert (P1) | Requires root (unlikely) | N/A (sandboxed) |
| Use VPN | WFP detects new tunnels (P2) | NetworkExtension detects (P2) | iptables + tun detection (P2) | VpnService conflict (P1) | NE conflict (P1) |
| Use proxy/SOCKS | WFP blocks non-filtered traffic (P2) | NE blocks proxy (P2) | iptables blocks (P2) | VpnService captures (P1) | NE captures (P1) |
| Use Tor | Block Tor nodes + detect binary (P2) | Block Tor nodes + detect (P2) | Block Tor nodes + detect (P2) | Block Tor app (P2) | Block Tor app (P2) |
| Tethering (this device provides) | WFP on this device still works (P1) | NE on this device still works (P1) | iptables on this device still works (P1) | VpnService on this device still works (P1) | NE on this device still works (P1) |
| IP-direct (bypass DNS) | IP blocklist via WFP (P2) | IP blocklist via NE (P2) | IP blocklist via iptables (P2) | IP blocklist via VpnService (P2) | IP blocklist via NE (P2) |

### 6.4 OS-Level Bypass

| Bypass Technique | Windows | macOS | Linux | Android | iOS |
|-----------------|---------|-------|-------|---------|-----|
| Boot safe mode | Boot-start WFP driver (P2) | N/A (no third-party safe mode) | N/A (no standard safe mode) | N/A | N/A |
| Boot recovery | BIOS password (P3 rec) | Recovery password (P3 rec) | GRUB password (P3 rec) | N/A | N/A |
| Boot from USB | BitLocker + BIOS password (P3) | FileVault + firmware password (P3) | LUKS + BIOS password (P3) | N/A | N/A |
| OS reinstall | Detect + alert (P1) | Detect + alert (P1) | Detect + alert (P1) | Knox survives reset (P3) | Supervision survives reset (P3) |
| Factory reset | N/A | N/A | N/A | Device Owner survives (P2) | Supervised mode survives (P3) |

### 6.5 Time and Clock Manipulation

| Bypass Technique | Windows | macOS | Linux | Android | iOS |
|-----------------|---------|-------|-------|---------|-----|
| System clock forward | Server-authoritative time (P1) | Server-authoritative time (P1) | Server-authoritative time (P1) | Server-authoritative time (P1) | Server-authoritative time (P1) |
| NTP manipulation | Clock drift detection (P1) | Clock drift detection (P1) | Clock drift detection (P1) | Clock drift detection (P1) | Clock drift detection (P1) |

### 6.6 Application-Level Bypass

| Bypass Technique | Windows | macOS | Linux | Android | iOS |
|-----------------|---------|-------|-------|---------|-----|
| Install gambling app | App blocking (P2) | App blocking (P2) | App blocking (P2) | App blocking (P2) | App blocking + Screen Time API (P3) |
| Use web-based gambling | DNS blocking covers this (P1) | DNS blocking (P1) | DNS blocking (P1) | DNS blocking (P1) | DNS blocking (P1) |
| Remove browser extension | Extension monitoring (P3) | Extension monitoring (P3) | Extension monitoring (P3) | N/A | N/A |
| Use incognito/private | Force extension in incognito (P3) | Force extension in incognito (P3) | Force extension in incognito (P3) | DNS still works (P1) | DNS still works (P1) |
| Use alternative browser | Browser monitoring (P3) | Browser monitoring (P3) | Browser monitoring (P3) | DNS still works (P1) | DNS still works (P1) |

### 6.7 Debugging / Reverse Engineering

| Bypass Technique | Windows | macOS | Linux | Android | iOS |
|-----------------|---------|-------|-------|---------|-----|
| Attach debugger | PPL process (P2) | Hardened runtime (P2) | ptrace_scope (P2) | Anti-debug (P2) | N/A (sandboxed) |
| Memory editing | PPL prevents (P2) | Hardened runtime prevents (P2) | ptrace_scope (P2) | Root required + detection (P2) | N/A (sandboxed) |
| Binary analysis | Strip symbols + obfuscation (P2) | Strip symbols + obfuscation (P2) | Strip symbols + obfuscation (P2) | Strip symbols + obfuscation (P2) | N/A (App Store) |

---

## 7. Supply Chain Security

### 7.1 Binary Signing and Verification Chain

```
[Developer Workstation]
        |
        v
[CI/CD Build System] -- produces --> [Unsigned Binary]
        |
        v
[Automated Build Key (HSM)] -- signs --> [Build-Signed Binary]
        |
        v
[Release Process (manual approval)]
        |
        v
[Offline Release Key (HSM, air-gapped)] -- countersigns --> [Dual-Signed Binary]
        |
        v
[Transparency Log (sigstore/Rekor)] <-- published
        |
        v
[CDN / Distribution] -- delivers --> [Agent on Device]
        |
        v
[Agent Verifies Both Signatures Against Embedded Public Keys]
```

**Key management:**

| Key | Storage | Access | Rotation |
|-----|---------|--------|----------|
| Build key (automated) | HSM attached to CI/CD | CI/CD system only, no human access to private key | Annually or on compromise |
| Release key (offline) | Air-gapped HSM | Requires 2-of-3 key custodians physically present | Every 2 years or on compromise |
| Agent verification keys (public) | Embedded in agent binary | Public, no secrecy required | Updated via signed agent update |
| mTLS CA key | HSM | API server only | Annually |
| Device client cert | Device hardware key store (TPM/Keychain/Keystore) | Agent only | Per enrollment |

### 7.2 Update Integrity Verification

The agent update process:

1. Agent checks for updates via API (periodic poll or push notification).
2. API responds with update metadata: version, size, SHA-256 hash, build signature, release signature, download URL.
3. Agent downloads the update binary from CDN.
4. Agent verifies:
   a. SHA-256 hash matches metadata.
   b. Build signature validates against embedded build public key.
   c. Release signature validates against embedded release public key.
   d. Version number is strictly greater than current version.
5. Agent applies update via platform-specific mechanism (replace binary and restart service).
6. On next startup, agent re-validates its own binary integrity.

**Rollback protection:** The agent rejects updates with a version number less than or equal to the current version. This prevents an attacker from "updating" to a vulnerable old version. Emergency rollbacks require a special API-authorized downgrade token that is time-limited and logged.

### 7.3 Dependency Supply Chain (Rust Crate Auditing)

- **cargo-audit:** Run `cargo audit` in CI/CD on every build. Fail the build on known vulnerabilities in dependencies.
- **cargo-deny:** Use `cargo deny` to enforce policies on dependency licenses, sources (crates.io only, no git dependencies in release builds), and known-bad crate versions.
- **cargo-vet:** Adopt Mozilla's `cargo-vet` for first-party and community-sourced audits of dependency versions. Every new dependency or version bump requires an audit before merge.
- **Dependency pinning:** `Cargo.lock` is committed and enforced. No floating version ranges in release builds.
- **Minimal dependencies:** Aggressively minimize the dependency tree. Prefer the Rust standard library over external crates where possible. Audit any crate with `unsafe` code.
- **SBOM generation:** Generate Software Bill of Materials (SBOM) in SPDX or CycloneDX format for every release. Publish alongside the binary.

### 7.4 Self-Hosted Blocklist Feed Integrity

- **Signed blocklist:** The community blocklist feed is cryptographically signed by BetBlocker's blocklist signing key. Self-hosted instances verify the signature before applying updates.
- **Feed integrity endpoint:** Self-hosted instances can verify the blocklist version and signature against a BetBlocker-hosted attestation endpoint (independent of the blocklist distribution channel).
- **Operator customization:** Self-hosted operators can add local blocklist entries (signed with their own key) but cannot modify or remove entries from the community feed without breaking the signature.
- **Feed poisoning detection:** The community feed is versioned and append-only. Entries are never removed (only deprecated/superseded). A self-hosted instance that receives a blocklist smaller than the previous version rejects it.

### 7.5 CI/CD Pipeline Security

- **Isolated build environment:** Builds run in ephemeral, hermetic containers. No network access during compilation (dependencies pre-fetched and cached).
- **Build reproducibility:** The build process is deterministic and reproducible. Build instructions are public. Anyone can verify a release.
- **Branch protection:** Main branch requires pull request review, passing CI (including security scans), and sign-off from a maintainer.
- **Secret management:** CI/CD secrets (signing keys, deployment credentials) stored in the platform's secret manager (GitHub Actions secrets, Vault). Never in code, environment files, or logs.
- **Dependency scanning:** Trivy and cargo-audit run on every PR. High/Critical vulnerabilities block merge.
- **SAST:** Semgrep with OWASP and CWE rulesets. Custom rules for BetBlocker-specific patterns (e.g., ensure all enrollment endpoints check authorization).
- **Secrets scanning:** Gitleaks on every PR and periodically on the full repository history. Pre-commit hooks for developer workstations.

---

## 8. Compliance Considerations

### 8.1 GDPR (EU Users)

| Requirement | BetBlocker Implementation |
|------------|--------------------------|
| **Lawful basis for processing** | Self-enrolled: consent (freely given at enrollment). Partner-enrolled: legitimate interest (accountability agreement between two parties). Authority-enrolled: legal obligation (court order). |
| **Right to access (Art. 15)** | Users can export all their data from the dashboard (account info, enrollment history, blocked-attempt logs, event history). API endpoint for programmatic access. |
| **Right to rectification (Art. 16)** | Users can update their account information. Blocked-attempt logs are system-generated and not subject to rectification (they are factual records). |
| **Right to erasure (Art. 17)** | Self-enrolled: full account and data deletion available after unenrollment. All data purged within 30 days. Partner-enrolled: requires partner agreement or "break glass" unenrollment first. Authority-enrolled: **conflict** -- court order may require data retention. See below. |
| **Right to data portability (Art. 20)** | Data export in machine-readable format (JSON). |
| **Data minimization (Art. 5(1)(c))** | See data inventory (Section 5.2). No browsing history, no location, no keystroke data. |
| **Storage limitation (Art. 5(1)(e))** | Configurable retention periods with automatic purging. |
| **Security (Art. 32)** | Encryption at rest and in transit, access controls, regular security audits, incident response procedures. |
| **Breach notification (Art. 33, 34)** | 72-hour notification to supervisory authority. Prompt notification to affected users. Documented incident response plan. |
| **Data Protection Impact Assessment (Art. 35)** | Required due to processing of health-related data (gambling addiction status is sensitive data under Art. 9). DPIA must be conducted before launch and updated with significant changes. |

**GDPR Conflict: Right to Erasure vs. Court-Ordered Retention**

When an authority tier enrollment is backed by a court order, the court order may require data retention for the duration of the legal proceeding or probation period. This conflicts with Art. 17 right to erasure.

Resolution: Art. 17(3)(b) exempts data processing required for compliance with a legal obligation. BetBlocker retains authority-tier data as required by the court order. The retention period is specified by the authority at enrollment. After the retention period expires, standard deletion procedures apply. The user is informed of the legal basis for retention and the retention period.

### 8.2 Data Residency (Institutional Tier)

- **Hosted platform:** Default deployment is in the EU (Frankfurt region) for EU users. US region available for US users. Configurable per organization for institutional tier.
- **Self-hosted:** The operator controls data residency entirely. Documentation includes guidance on deploying in specific jurisdictions.
- **Cross-border considerations:** Partner relationships may cross borders (partner in EU, enrolled user in US). Data processing agreements must account for this.
- **Institutional requirements:** Some courts and government programs require data to remain within their jurisdiction. Institutional tier supports organization-level data residency configuration. For hosted platform, this may require region-specific database instances.

### 8.3 Audit Log Accessibility for Court Proceedings

- **Tamper-evident logs:** Authority-tier audit logs are stored in an append-only format with cryptographic chaining (each entry includes the hash of the previous entry). This provides evidence of completeness -- a missing entry breaks the chain.
- **Export format:** Audit logs can be exported in a standardized, court-admissible format with timestamps, event descriptions, and integrity verification data.
- **Third-party attestation (future):** Investigate integration with a third-party timestamping service (RFC 3161) or blockchain-anchored timestamping to provide independent proof of log integrity.
- **Chain of custody:** Documentation covers the chain of custody for audit log exports, including who generated the export, when, and how the integrity can be independently verified.
- **Retention:** Authority-tier audit logs are retained for the duration specified by the authority, with a minimum of 7 years for court-related enrollments (configurable).

### 8.4 Additional Regulatory Considerations

| Regulation | Relevance | Notes |
|-----------|-----------|-------|
| **CCPA/CPRA (California)** | Applies to California users of the hosted platform | Similar to GDPR. Right to delete, right to know, right to opt out of sale (BetBlocker does not sell data). |
| **HIPAA (US Healthcare)** | May apply if BetBlocker is used as part of a healthcare treatment program | If therapy practices use BetBlocker, the data may be PHI. Institutional tier must support BAA (Business Associate Agreement) with healthcare providers. |
| **SOC 2** | Expected by institutional customers | Pursue SOC 2 Type II certification for the hosted platform. Covers security, availability, processing integrity, confidentiality, privacy. |
| **Gambling regulations** | Varies by jurisdiction | BetBlocker is not a gambling operator and is not subject to gambling regulations. However, some jurisdictions have self-exclusion programs that BetBlocker could integrate with. Legal review required per jurisdiction. |
| **Accessibility (WCAG)** | Applies to the web platform | The web platform must be WCAG 2.1 AA compliant. This is both a legal requirement (ADA, EAA) and an ethical imperative for a platform serving a vulnerable population. |

---

## 9. Risk Summary and Prioritization

### 9.1 Critical Risks (Immediate Action Required)

| ID | Threat | Risk | Primary Mitigation | Phase |
|----|--------|------|-------------------|-------|
| T-002 | Account takeover / credential theft | Critical | MFA, rate limiting, breach password check | P1 |
| T-004 | Agent binary replacement | Critical | Hash validation, watchdog, kernel protection | P1/P2 |
| T-006 | Blocklist cache tampering | Critical | Signed blocklist, in-memory validation | P1 |
| T-007 | DNS configuration bypass (DoH/DoT, HOSTS) | Critical | Platform network hooks, DNS enforcement | P1/P2 |
| T-016 | Agent process termination | Critical | System service, watchdog, persistent rules | P1/P2 |
| T-019 | OS reinstallation / factory reset | Critical | Detection + alerting; MDM/Knox for mobile | P1/P3 |
| T-020 | Privilege escalation to disable agent | Critical | Kernel-level protections, assume admin | P1/P2 |
| T-021 | Enrollment tier escalation (API) | Critical | RBAC, enrollment-aware auth | P1 |
| T-023 | VPN/proxy/Tor bypass | Critical | VPN detection, app blocking, network hooks | P1/P2 |
| T-026 | Agent update hijacking | Critical | Dual signing, staged rollout, transparency | P1/P2 |
| T-033 | Broken access control (IDOR) | Critical | Authorization middleware, automated testing | P1 |

### 9.2 High Risks (Address in Current Phase)

| ID | Threat | Risk | Primary Mitigation | Phase |
|----|--------|------|-------------------|-------|
| T-001 | Agent identity spoofing | High | mTLS, device certificates, attestation | P1/P2 |
| T-005 | Agent configuration tampering | High | Hardware-bound encryption, signing | P1/P2 |
| T-009 | Time manipulation for unenrollment | High | Server-authoritative time | P1 |
| T-010 | Tamper event suppression | High | Watchdog events, heartbeat digest, offline alert | P1/P2 |
| T-014 | Enrollment relationship disclosure | High | No enumeration, constant-time auth, encryption | P1 |
| T-015 | Self-hosted operator data access | High | Disclosure, password hashing, E2E encryption | P1/P3 |
| T-022 | Safe mode / recovery mode bypass | High | Post-boot integrity, persistent driver, BIOS | P1/P2/P3 |
| T-025 | Reverse engineering the agent | High | Kerckhoffs' principle, no embedded secrets | P1/P2 |
| T-032 | CSRF (especially partner approval) | High | SameSite, CSRF tokens, re-auth for critical actions | P1 |
| T-035 | Session hijacking | High | Short-lived tokens, rotation, binding | P1 |
| PT-001 | Surveillance abuse by partner | High | Break glass, aggregated reports, delay, abuse reporting | P1 |

### 9.3 Medium Risks (Address in Next Phase)

| ID | Threat | Risk | Primary Mitigation | Phase |
|----|--------|------|-------------------|-------|
| T-003 | Partner invitation spoofing | Medium | Single-use tokens, authenticated acceptance | P1 |
| T-008 | Blocklist poisoning via federated reports | Medium | Reputation system, human review, thresholds | P2 |
| T-011 | Non-repudiation of partner actions | Medium | Signed action log, confirmations | P1/P3 |
| T-012 | Browsing history inference from logs | Medium | Aggregated reports, data minimization, retention | P1 |
| T-017 | Device offline / network isolation | Medium | Offline blocking, heartbeat alerting, event queue | P1/P2 |
| T-018 | DDoS against central API | Medium | CDN/WAF, rate limiting, agent resilience | P1/P2 |
| T-028 | Browser-based bypass | Medium | Multi-layer blocking, extension monitoring | P3 |
| T-029 | Debugging and memory manipulation | Medium | Anti-debug, PPL, hardened runtime | P2 |
| T-030 | SQL injection | Medium | Parameterized queries, SAST | P1 |
| T-031 | XSS | Medium | React escaping, CSP | P1 |
| T-034 | SSRF | Medium | URL allowlisting, network segmentation | P1/P2 |
| PT-004 | Data breach exposure | Medium | Encryption, minimization, segmentation | P1 |

### 9.4 Low Risks (Monitor and Address Opportunistically)

| ID | Threat | Risk | Primary Mitigation | Phase |
|----|--------|------|-------------------|-------|
| T-013 | API error message leakage | Low | Generic errors, structured logging | P1 |
| T-027 | Tethering bypass | Low | Device-level hooks handle this by design | P1 |
| PT-003 | Federated report deanonymization | Low | Stripping, batching, aggregation threshold | P2 |

### 9.5 Accepted / Out-of-Scope Risks

| ID | Threat | Rationale for Acceptance |
|----|--------|------------------------|
| T-024 | Use of a separate device | Fundamentally unsolvable by endpoint software. Mitigated by multi-device enrollment UX and partner/authority awareness. |
| -- | Nation-state targeted attack | BetBlocker is not a primary nation-state target. Institutional tier inherits the security posture of the platform, which is designed to resist TA-4 through TA-7. Additional nation-state-specific hardening is not cost-effective at this stage. |
| -- | Hardware implant attacks | Out of scope for a software product. |
| -- | Quantum cryptanalysis | Current cryptographic algorithms are sufficient. Monitor NIST post-quantum standardization and adopt PQC when mature. |

---

## 10. Appendix: Threat Registry

### 10.1 Full Threat Index

| ID | Category | Description | Actor | Likelihood | Impact | Risk | Phase |
|----|----------|-------------|-------|-----------|--------|------|-------|
| T-001 | Spoofing | Agent identity spoofing | TA-2,3 | Med | High | High | P1/P2 |
| T-002 | Spoofing | Account takeover | TA-1,4 | High | Crit | Crit | P1 |
| T-003 | Spoofing | Partner invitation spoofing | TA-4,5 | Low | High | Med | P1 |
| T-004 | Tampering | Agent binary replacement | TA-1,2,3 | Med | Crit | Crit | P1/P2 |
| T-005 | Tampering | Agent configuration tampering | TA-1,2,3 | Med | High | High | P1/P2 |
| T-006 | Tampering | Blocklist cache tampering | TA-1,2,3 | Med | Crit | Crit | P1 |
| T-007 | Tampering | DNS configuration bypass | TA-1,2,3 | High | Crit | Crit | P1/P2 |
| T-008 | Tampering | Blocklist poisoning (federated) | TA-4,6 | Med | Med | Med | P2 |
| T-009 | Tampering | Time manipulation for unenrollment | TA-1 | Med | High | High | P1 |
| T-010 | Repudiation | Tamper event suppression | TA-2,3 | Med | High | High | P1/P2 |
| T-011 | Repudiation | Non-repudiation of partner actions | TA-5 | Low | Med | Med | P1/P3 |
| T-012 | Info Disclosure | Browsing history inference | TA-5,6 | High | Med | Med | P1 |
| T-013 | Info Disclosure | API error leakage | TA-4 | Med | Low | Low | P1 |
| T-014 | Info Disclosure | Enrollment relationship disclosure | TA-4,5 | Med | High | High | P1 |
| T-015 | Info Disclosure | Self-hosted operator data access | TA-6 | High | High | High | P1/P3 |
| T-016 | DoS | Agent process termination | TA-1,2,3 | High | Crit | Crit | P1/P2 |
| T-017 | DoS | Device offline / network isolation | TA-1,2,3 | Med | Med | Med | P1/P2 |
| T-018 | DoS | DDoS against API | TA-4 | Med | Med | Med | P1/P2 |
| T-019 | DoS | OS reinstallation / factory reset | TA-1,2,3 | Med | Crit | Crit | P1/P3 |
| T-020 | EoP | Privilege escalation to disable agent | TA-1,2,3 | High | Crit | Crit | P1/P2 |
| T-021 | EoP | Enrollment tier escalation (API) | TA-1-4 | Med | Crit | Crit | P1 |
| T-022 | EoP | Safe mode / recovery mode | TA-1,2,3 | Med | High | High | P1/P2/P3 |
| T-023 | Tampering | VPN/proxy/Tor bypass | TA-1,2,3 | High | Crit | Crit | P1/P2 |
| T-024 | Out of band | Separate device | TA-1,2,3 | Very High | Crit | Accepted | P1 |
| T-025 | Mixed | Reverse engineering agent | TA-2-4,7 | Med | High | High | P1/P2 |
| T-026 | Tampering/EoP | Agent update hijacking | TA-4,7 | Low | Crit | Crit | P1/P2 |
| T-027 | Tampering | Tethering bypass | TA-1,2,3 | Med | Low | Low | P1 |
| T-028 | Tampering | Browser-based bypass | TA-1,2,3 | High | Med | Med | P3 |
| T-029 | Tampering/EoP | Debugging / memory manipulation | TA-2,3 | Low | High | Med | P2 |
| T-030 | Tampering | SQL injection | TA-4 | Low | Crit | Med | P1 |
| T-031 | Tampering | XSS | TA-4 | Low | High | Med | P1 |
| T-032 | Spoofing | CSRF | TA-1,4 | Med | High | High | P1 |
| T-033 | EoP | Broken access control (IDOR) | TA-1,2,4 | Med | Crit | Crit | P1 |
| T-034 | Tampering | SSRF | TA-4 | Low | High | Med | P1/P2 |
| T-035 | Spoofing | Session hijacking | TA-1,4 | Med | High | High | P1 |
| PT-001 | Privacy | Surveillance abuse by partner | TA-5 | High | High | High | P1 |
| PT-002 | Privacy | Platform as spyware | TA-6,7 | Low | Crit | Med | P1/P4 |
| PT-003 | Privacy | Federated report deanonymization | TA-4,6 | Low | Med | Low | P2 |
| PT-004 | Privacy | Data breach exposure | TA-4 | Med | High | Med | P1 |

### 10.2 Review History

| Date | Reviewer | Changes |
|------|---------|---------|
| 2026-03-12 | JD + Claude | Initial threat model |

### 10.3 Next Review Actions

- Conduct tabletop exercises for T-002 (account takeover) and T-026 (update hijacking) scenarios.
- Validate anti-bypass matrix against actual platform API capabilities for each OS.
- Commission external penetration test after Phase 1 launch targeting T-021 (authorization) and T-033 (access control).
- Conduct DPIA (Data Protection Impact Assessment) for GDPR compliance before processing EU user data.
- Establish bug bounty program to incentivize external security research.
