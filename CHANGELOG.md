# Changelog

## [0.2.0](https://github.com/JerrettDavis/BetBlocker/compare/v0.1.0...v0.2.0) (2026-03-17)


### Features

* add Astro + Starlight GitHub Pages site with marketing landing and docs ([490e3b3](https://github.com/JerrettDavis/BetBlocker/commit/490e3b3bf9f39b8e4ef147df81f9300225220cd5))
* add stub crates for API, worker, agent, and CLI ([73c153f](https://github.com/JerrettDavis/BetBlocker/commit/73c153fbafff9a25d2f1680341a308b1af1e2757))
* **agent:** add API client, registration, heartbeat, blocklist sync, event reporter, watchdog, and integrity checking ([692bf63](https://github.com/JerrettDavis/BetBlocker/commit/692bf633328705ceb51dc67390eda9ac1873e005))
* **agent:** add Linux MAC integration, app process integration tests, and config validation (SP7 T6, SP3 T24-26) ([162f841](https://github.com/JerrettDavis/BetBlocker/commit/162f8415653cd498901554930c63fcc205b0315f))
* **agent:** implement agent core engine with plugin system, blocklist, events, and config ([b0d4089](https://github.com/JerrettDavis/BetBlocker/commit/b0d408966bf03bfc40fc5b761c04d8c18325683a))
* **analytics,review-queue:** add TimescaleDB analytics and discovery review queue (SP1 T13-16, SP2 T11-12) ([8232200](https://github.com/JerrettDavis/BetBlocker/commit/8232200f1827263881a9f794855353459a9d573a))
* **api,web:** add analytics export and enhanced dashboard (SP1 Tasks 19-22) ([8fab681](https://github.com/JerrettDavis/BetBlocker/commit/8fab681f0d7f23a232ab680989f60a90ad2fce4f))
* **api,worker,agent:** add federated ingestion pipeline, Tor exit node sync, bypass detection feature flags (SP2 T21-27, SP4 T14-19) ([e6a9cd9](https://github.com/JerrettDavis/BetBlocker/commit/e6a9cd9a889ea8b851d3e2918993162b28486728))
* **api:** implement complete Axum API server (Chunks 1-5) ([594b29a](https://github.com/JerrettDavis/BetBlocker/commit/594b29a0e473fea2e9453c025f111888afd57d8a))
* **bb-agent-core:** add federated reporting with token rotation and temporal bucketing (SP2 Tasks 19-20) ([15aca82](https://github.com/JerrettDavis/BetBlocker/commit/15aca82ecc4580f2d545d94182a32c134f3886f3))
* **bb-agent-plugins:** add AppProcessPlugin with interceptor, install watcher, and registry integration (SP3 Tasks 11-20) ([f035753](https://github.com/JerrettDavis/BetBlocker/commit/f03575375e0a829d3dfb2b622f248a11b05b4d30))
* **bb-shim-linux:** implement MAC detection, AppArmor profiles, and installation automation (SP7 Tasks 1-3) ([5cc65a2](https://github.com/JerrettDavis/BetBlocker/commit/5cc65a25c9ad27be2ed25a3f77fc41a5147bdb31))
* **bb-shim-windows:** implement Windows Service lifecycle management ([aee3c8a](https://github.com/JerrettDavis/BetBlocker/commit/aee3c8a02ce14926f9279fdef7ce6e89f1c265c0))
* **bb-worker:** add registry, WHOIS, DNS zone, and search crawlers (SP2 Tasks 14-18) ([3860bb8](https://github.com/JerrettDavis/BetBlocker/commit/3860bb85ed196bfc7ea0d5f10d6e9e375055a109))
* **bb-worker:** add trend analysis engine and content classifier (SP1 T17-18, SP2 T9-10) ([e90a6a7](https://github.com/JerrettDavis/BetBlocker/commit/e90a6a74d17b2ba226f8fb789f86eb73159afbb8))
* **bb-worker:** implement discovery pipeline worker (SP2 Tasks 5-8) ([0634c5d](https://github.com/JerrettDavis/BetBlocker/commit/0634c5def5863ffaea54b6cdbeb94070d3884c9b))
* **bypass-detection:** add Linux bypass detection and response orchestrator (SP4 Tasks 8-13) ([6a30f15](https://github.com/JerrettDavis/BetBlocker/commit/6a30f1507876327ff4d9d8c8cbdf8506b37eafa5))
* **bypass-detection:** add VPN, proxy, and Tor detection module (SP4 Tasks 3-7) ([41fdfcf](https://github.com/JerrettDavis/BetBlocker/commit/41fdfcfb2333f5754b76915574937af62ecce53d))
* **ci:** add merge and release workflows, enhance CI with Dockerfile linting ([af4f1a9](https://github.com/JerrettDavis/BetBlocker/commit/af4f1a9411cedcd634946c01a30cf4af27fcb011))
* **common:** add bb-common crate with domain enums, error types, and models ([0504343](https://github.com/JerrettDavis/BetBlocker/commit/0504343a85d9354c7de6f8744e5471d452647213))
* **db:** add all Phase 1 database migrations (V001-V021) ([8ee7633](https://github.com/JerrettDavis/BetBlocker/commit/8ee7633adb2f18c451681bda8f7d1b3429fa6154))
* **deploy:** add Dockerfiles for web dashboard and Linux agent ([628dded](https://github.com/JerrettDavis/BetBlocker/commit/628dded448ba7313db6933e50ab0678cf22635ff))
* **deploy:** add Ed25519 binary signing and verification scripts ([355bbf4](https://github.com/JerrettDavis/BetBlocker/commit/355bbf416fc9df7d9f091ca5c04506ab4d58d490))
* **deploy:** add Helm chart skeleton for hosted Kubernetes deployment ([0b8117e](https://github.com/JerrettDavis/BetBlocker/commit/0b8117edb3839dc544acb88e17ad1c269de803bc))
* **deploy:** add idempotent self-hosted setup script ([a727798](https://github.com/JerrettDavis/BetBlocker/commit/a7277980f0b79345652f216d27183f2cee403cd3))
* **deploy:** add multi-stage Dockerfiles for API and worker ([9378cfb](https://github.com/JerrettDavis/BetBlocker/commit/9378cfb2dc3024242681330952eee11b22b6ab99))
* **deploy:** add production docker-compose for self-hosted deployment ([7c25934](https://github.com/JerrettDavis/BetBlocker/commit/7c2593487b58cb8e5f046894be61f682f3694ee8))
* **linux:** add Linux platform integration with systemd, nftables, and agent entrypoint ([ac802f9](https://github.com/JerrettDavis/BetBlocker/commit/ac802f927c583cf14bcf63cc404044ccb08ee8fb))
* **macos:** add XPC bridge, installer helpers, and pkg build scripts (SP6 Chunks 4-5) ([6c3a84b](https://github.com/JerrettDavis/BetBlocker/commit/6c3a84bd1d592977573d06770d0fc53d73b1061c))
* **macos:** implement macOS platform support (SP6 Chunks 1-2) ([02d2388](https://github.com/JerrettDavis/BetBlocker/commit/02d2388f2e51bac6b0243576ced2058c52574b8a))
* **mobile:** add Android Device Owner, Knox, VPN service, and iOS MDM/content filter stubs (SP7 Tasks 8-14) ([c89a2d6](https://github.com/JerrettDavis/BetBlocker/commit/c89a2d64d523ab6aad4f653a2eb234d6da8a5d7d))
* **phase2:** add org members, foundation models, and platform crate scaffolding ([41f2a10](https://github.com/JerrettDavis/BetBlocker/commit/41f2a10475a35dd8a0cf0e99fcdfd3c318e84ca4))
* **phase2:** add organization CRUD - migrations, models, service, routes ([13a091c](https://github.com/JerrettDavis/BetBlocker/commit/13a091c9896c54b2499863b0198b663530e2507b))
* **phase2:** add organization member management + TypeScript types ([184ac90](https://github.com/JerrettDavis/BetBlocker/commit/184ac90c6bd3d7e0928887a08dc0e279dffc8854))
* **phase2:** scaffold platform shim and agent crates (windows, macos, linux, android, ios) ([b8ff142](https://github.com/JerrettDavis/BetBlocker/commit/b8ff1429aef47042b801f0dba5be02f4de280c49))
* **platform:** implement platform-specific security features (SP5-SP7) ([61f8807](https://github.com/JerrettDavis/BetBlocker/commit/61f88078a932972875ebb73a1e7a3202f0a66c60))
* **proto:** add protobuf definitions for agent-API protocol ([de67cd8](https://github.com/JerrettDavis/BetBlocker/commit/de67cd87f097c12f62c1be774c36c9c4915ba1a4))
* **web:** add complete Next.js web application for BetBlocker ([a9f9cc2](https://github.com/JerrettDavis/BetBlocker/commit/a9f9cc2de8ae9c22d7c052760d3c2974f90fcacf))
* **web:** add organization management pages and navigation (SP1 Tasks 11-12) ([19a24e3](https://github.com/JerrettDavis/BetBlocker/commit/19a24e3ff8c390de24b648aaa1c6d1961bd9bc0b))
* **web:** add review queue and app signature admin UI (SP2 T13, SP3 T21-23) ([31dfd9a](https://github.com/JerrettDavis/BetBlocker/commit/31dfd9a65bc8428921ab89e2d33a057d0a01663e))
* **windows:** add WFP and minifilter IOCTL interfaces and driver stubs (SP5 Tasks 14-17) ([aa92c85](https://github.com/JerrettDavis/BetBlocker/commit/aa92c855b51be2676ace39ca94bff7cf0e1e7547))
* **windows:** add Windows agent binary, MSI installer, and auto-updater (SP5 Tasks 8-13) ([fc807a8](https://github.com/JerrettDavis/BetBlocker/commit/fc807a8a9f50771f289428f5dd44326e4a3f773b))


### Bug Fixes

* add VPN Block mode variant and macOS installer scripts (I2, I5) ([725dcd7](https://github.com/JerrettDavis/BetBlocker/commit/725dcd7bc22d5bf8845027915d9e3679b4eb6618))
* align federated service with federated_reports_v2 table schema (C1) ([1726feb](https://github.com/JerrettDavis/BetBlocker/commit/1726feb87339d2d8aca2667d3192ef82a37a15b2))
* align migrations with sqlx format, expand blocklist, add Phase 2 design ([c5a2e00](https://github.com/JerrettDavis/BetBlocker/commit/c5a2e00b1c036506c69cb0cfb306c88d79878bc9))
* **ci:** apply cargo fmt and fix musl linker in Dockerfiles ([d6fb8b5](https://github.com/JerrettDavis/BetBlocker/commit/d6fb8b5910704cef90d4f8de29ee681ea076643c))
* **ci:** fix bypass_detection test assertions and add missing clippy allows ([b54d826](https://github.com/JerrettDavis/BetBlocker/commit/b54d826e37bb636b8d79ad6ef4b4e2f24e9d2c73))
* **ci:** fix compilation errors on Linux CI and lowercase Docker image names ([505b1b0](https://github.com/JerrettDavis/BetBlocker/commit/505b1b0d384c181a41bb096c49368900d5f2604d))
* **ci:** resolve all four CI job failures ([ed4b481](https://github.com/JerrettDavis/BetBlocker/commit/ed4b4810f5a8144285a454796036187a50d6e0a4))
* **ci:** resolve clippy pedantic lints and replace TimescaleDB continuous aggregates ([68dcb1c](https://github.com/JerrettDavis/BetBlocker/commit/68dcb1c9cbbb0117d6a4e6143f8a52fd53971074))
* **ci:** resolve clippy, migration, Docker build, and binary name issues ([d4a4cb0](https://github.com/JerrettDavis/BetBlocker/commit/d4a4cb0c1023b3f44a80c3a35ee3ebd1b702ccd5))
* **ci:** switch release-please to simple strategy for workspace version inheritance ([fd02053](https://github.com/JerrettDavis/BetBlocker/commit/fd02053b8ac56af0554e87a147d654cafbe7cf53))
* recreate migration files with sqlx-compatible naming and align schema with API code ([05b5562](https://github.com/JerrettDavis/BetBlocker/commit/05b5562a0f551ac382cb901e833eef42c02714e2))
* resolve bb-api integration test DB connection on CI ([511f386](https://github.com/JerrettDavis/BetBlocker/commit/511f38681bb8b53971e6482a1ea139d9fcee6787))
* resolve CI test failure and merge workflow issues ([ae94e73](https://github.com/JerrettDavis/BetBlocker/commit/ae94e732eeed0ce07b78aae9fc4775cb0655b2d8))
* resolve compile errors from Wave 4 integration ([629a141](https://github.com/JerrettDavis/BetBlocker/commit/629a141649f8c76c91320be6c82905e321d66dfb))
* **site:** make logo and nav readable in dark mode ([0d2800e](https://github.com/JerrettDavis/BetBlocker/commit/0d2800e2a3482c5a041e0e175009bac80dd4893b))
* **site:** normalize BASE_URL trailing slash in all component links ([7e0c9f7](https://github.com/JerrettDavis/BetBlocker/commit/7e0c9f74c0ed8d88ec2fc94d37cb3e4cf4f61133))
* skip macOS installer tests on non-macOS CI ([49577a6](https://github.com/JerrettDavis/BetBlocker/commit/49577a6e0ba9dd0def2d60cbee463af1796b907e))
* sync TypeScript types with Rust enums (C2, C3, M1, M2) ([55f7eff](https://github.com/JerrettDavis/BetBlocker/commit/55f7effd7be6f1c52a05a767bc6808b034dfe839))
* wire discovery classifier/scorer and fix federated report serialization (I3, I6) ([c7116d0](https://github.com/JerrettDavis/BetBlocker/commit/c7116d0eb870eda9e4033f7750ad80a3245278c5))


### Documentation

* add BetBlocker vision and design document ([a241fdf](https://github.com/JerrettDavis/BetBlocker/commit/a241fdf042e39f22dbc62104c723e0087abd1cda))
* add complete architecture package for BetBlocker ([360398c](https://github.com/JerrettDavis/BetBlocker/commit/360398ca6db926dbc1cd6deb65a24ad7887c048f))
* add Phase 1 implementation plans (7 sub-plans, ~8800 lines) ([d5c71a2](https://github.com/JerrettDavis/BetBlocker/commit/d5c71a2074cbe0302d5812c337d57f38e8a8e0c2))
* add Phase 2 implementation plans (master + 7 sub-plans) ([ab0cf22](https://github.com/JerrettDavis/BetBlocker/commit/ab0cf22600773df9aa4a543b3f5778ac2f349016))
* add README, LICENSE, contributing guide, and project documentation ([a7a8a51](https://github.com/JerrettDavis/BetBlocker/commit/a7a8a51651604d7592062b45c2b887a2c0bb1a13))


### Miscellaneous

* add dev docker-compose with PostgreSQL/TimescaleDB and Redis ([8b5e3ec](https://github.com/JerrettDavis/BetBlocker/commit/8b5e3ecd12f7f2737bd283e07398e5025b0d6003))
* add justfile dev commands and GitHub Actions CI ([49683a4](https://github.com/JerrettDavis/BetBlocker/commit/49683a41ec8f855b3b5a2f530275a094faa7c567))
* initialize Cargo workspace and toolchain config ([0be69d6](https://github.com/JerrettDavis/BetBlocker/commit/0be69d6d86cf9c04e6f8b2cde0608d94519b156f))
* suppress dead_code warnings for future-use items in bb-worker ([19da163](https://github.com/JerrettDavis/BetBlocker/commit/19da163f3725943ca8bd4191d3ea3f8de349cfa2))


### CI/CD

* add semver release pipeline with release-please, multi-platform packaging, and PR title validation ([8f4ddc5](https://github.com/JerrettDavis/BetBlocker/commit/8f4ddc5ba3f450b441729ad240f71930c374159e))
