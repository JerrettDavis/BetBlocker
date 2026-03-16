# Introducing BetBlocker: Open-Source, Multi-Layer Gambling Blocking for Every Platform

---

## Why We Built BetBlocker

Online gambling has never been more accessible. It runs on every device, follows you through every app, and operates around the clock. For the millions of people working toward recovery from gambling addiction, that constant accessibility is the hardest part. A single moment of vulnerability — late at night, after a bad day, during a trigger — can undo months of progress.

The tools that exist today are not good enough. Most gambling blockers rely on basic DNS filtering: a single layer of protection that can be bypassed by switching DNS providers, installing a VPN, or simply opening a gambling app instead of a website. Some collect browsing data. Most charge subscription fees that put protection out of reach. And almost none of them give the people who support recovery — therapists, sponsors, family members — any meaningful visibility into whether blocking is actually working.

We built BetBlocker because recovery deserves infrastructure-grade protection, not a browser plugin.

## What Makes BetBlocker Different

**Open source, top to bottom.** BetBlocker's entire codebase — the endpoint agent, the central API, the web platform, the blocklist pipeline — is open source. Every privacy claim is verifiable. Every security mechanism is auditable. If you do not trust us, you do not have to — you can read the code, build it yourself, and host it on your own infrastructure. The self-hosted version is free and includes every feature.

**Three independent blocking layers.** BetBlocker does not depend on a single point of defense. Layer one: DNS and network filtering intercepts gambling domains before they load, across every app and browser, including those with hardcoded DNS settings. Layer two: application blocking detects, prevents the launch of, and blocks the installation of known gambling apps. Layer three: browser extensions scan page content for gambling elements — ads, affiliate links, promotions — even on domains that are not themselves gambling sites.

If one layer is bypassed, the others hold. That redundancy is the point.

**Real tamper resistance.** BetBlocker runs as a system-level service with mutual watchdog processes, binary integrity checks, and platform-specific kernel protections. On Windows, a WFP driver maintains blocking even if the agent is terminated. On macOS, System Extensions require admin access and a reboot to remove. Unenrollment is never instant — it requires either a time delay or accountability partner approval, creating the deliberate pause that separates an impulsive decision from a considered one.

**Accountability built in.** BetBlocker supports three enrollment tiers: self-enrolled (with time-delayed unenrollment), partner-enrolled (requiring partner approval for changes), and institutional (for court-mandated programs and treatment providers). Accountability partners get their own dashboard showing device status, blocking health, and tamper alerts — without seeing browsing history or personal activity. Treatment programs can manage enrollments across dozens of clients from a centralized panel with full audit trails.

**Privacy as architecture, not policy.** No keylogging, no screen capture, no location tracking, no browsing history collection, no data sales. BetBlocker collects only the minimum gambling-domain metadata needed for blocking intelligence, and even that is configurable. Self-hosted deployments keep all data on your own infrastructure with zero phone-home behavior.

## Under the Hood

The endpoint agent is written entirely in Rust, compiled natively for Windows, macOS, Linux, Android, and iOS from a single codebase. Platform-specific integrations — Windows Filtering Platform, macOS Network Extensions, Android VPN Service, Linux nftables — are thin native shims that plug into the cross-platform core.

The central API (Rust, Axum) is a single stateless binary that handles authentication, enrollment management, blocklist distribution, and event ingestion. The web platform (Next.js) provides dashboards for users, partners, and institutions. Everything ships as Docker containers — the hosted platform runs the exact same images that self-hosters pull.

Blocklist intelligence is federated. Every enrolled agent contributes anonymized metadata about blocked domains back to a central pipeline (with explicit opt-in for self-hosted users). Automated classifiers process reports, and new gambling domains are promoted to the blocklist after human review. The blocklist is versioned, cryptographically signed, and delivered as incremental deltas to minimize bandwidth.

## What BetBlocker Does Not Do

We want to be direct about boundaries. BetBlocker is not spyware — it does not monitor anything outside of gambling blocking. It is not a general content filter — it blocks gambling, and only gambling. It does not lock users out of their devices. It does not require an internet connection to block (the full blocklist is cached locally). And it does not sell data, ever.

## Get Involved

**If you are in recovery** and want protection that takes your recovery as seriously as you do: download BetBlocker for your platform and enroll in under two minutes.

**If you are a developer** — especially one with experience in Rust, systems programming, security tooling, or cross-platform development: the entire project is on GitHub. We welcome contributions to the agent, the API, the web platform, and especially the blocklist pipeline.

**If you work in treatment or recovery support** — as a therapist, counselor, or program administrator: the institutional tier is built for you. Contact us to discuss how BetBlocker can integrate with your practice.

**If you know someone who might benefit:** share this post. Recovery tools only work when people know they exist.

BetBlocker is free, open source, and built for the long term. We believe that everyone working toward recovery from gambling addiction deserves tools that hold firm when they need them most.

**Download BetBlocker** | **View on GitHub** | **Read the Documentation**

---

*BetBlocker is not a substitute for professional treatment. If you or someone you know is struggling with gambling addiction, please contact the National Problem Gambling Helpline at 1-800-522-4700.*
