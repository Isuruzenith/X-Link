# Changelog

## [0.8.0](https://github.com/Isuruzenith/X-Link/compare/x-link-v0.7.2...x-link-v0.8.0) (2026-07-14)


### Features

* implement core application layout with navigation rail, UI components, and routing views ([d1b9722](https://github.com/Isuruzenith/X-Link/commit/d1b97226b5d90ccaa7a340825898f6f3f56c394a))
* implement core application shell and primary dashboard views for X-Link ([dbd0764](https://github.com/Isuruzenith/X-Link/commit/dbd07647fdcb2b00a1ecb361b828c69fbd6f3cc0))
* implement core application views and unified shell component structure ([dc5d26e](https://github.com/Isuruzenith/X-Link/commit/dc5d26eae0b46eaa05d3b91c755623de0e5946a1))
* implement project scaffolding, core Rust commands, and landing page assets ([e3c8ca9](https://github.com/Isuruzenith/X-Link/commit/e3c8ca98462fbf72eb242647ab43e826bedb44f8))
* implement settings UI with modular config, traffic monitoring components, and toast notifications ([8c5b3b4](https://github.com/Isuruzenith/X-Link/commit/8c5b3b418046b222051027dc5538866272526233))
* implement traffic visualization, notification system, and foundational application views ([2990f47](https://github.com/Isuruzenith/X-Link/commit/2990f473669f7e50f8fefc4c2575d057c4670baf))
* initialize project architecture with ViewShell, UI component library, and core configuration views ([19221b9](https://github.com/Isuruzenith/X-Link/commit/19221b9ec45ba13d70d8441a3c9b970d5df8d686))
* initialize project structure with core layout, design tokens, and base UI components ([4ef502b](https://github.com/Isuruzenith/X-Link/commit/4ef502b34ab3c492e04a2cf33d33e1704273c933))

## [0.7.2](https://github.com/Isuruzenith/X-Link/compare/x-link-v0.7.1...x-link-v0.7.2) (2026-07-04)


### Bug Fixes

* implement configuration generator and infrastructure for persistent application state and migration management ([ae75429](https://github.com/Isuruzenith/X-Link/commit/ae75429f1a5baa4d031178784e50d460d5bc8613))
* implement configuration management utilities for routing rules, TUN compatibility, and system DNS resolution ([5d1ef7d](https://github.com/Isuruzenith/X-Link/commit/5d1ef7d51e40edb14f9bb7ee9111eecf0350aff5))
* implement core data storage, configuration schema migration, and backend state management utilities. ([c83e0a9](https://github.com/Isuruzenith/X-Link/commit/c83e0a928de49586c1f294be0da280977dce76dc))
* implement persistent settings management and system configuration state sync ([31c520e](https://github.com/Isuruzenith/X-Link/commit/31c520eaa31ee2af277767c1e75021dbdadee4b6))
* implement system tray menu and management utilities in tray.rs ([379b063](https://github.com/Isuruzenith/X-Link/commit/379b063983909f5bd79ea10a7aefd9824ab45903))

## [0.7.1](https://github.com/Isuruzenith/X-Link/compare/x-link-v0.7.0...x-link-v0.7.1) (2026-07-04)


### Bug Fixes

* bypass local FakeIP DNS hijack in latency testing and server resolution to avoid false 0ms pings ([1260dd8](https://github.com/Isuruzenith/X-Link/commit/1260dd838857c3df209cff9645d08a6ef58298e1))
* exclude IPv6 loopback from TUN routing and use explicit 127.0.0.1 devUrl to prevent dev mode white screen ([266c7d8](https://github.com/Isuruzenith/X-Link/commit/266c7d806276951ad3e2a543a1a116bfa9c8b82b))
* implement configuration management module and latency command utilities for Tauri integration ([aae9b03](https://github.com/Isuruzenith/X-Link/commit/aae9b03a496cd3ea06a8d3caf4b9b6bf3bb53c24))
* implement node latency testing with DoH support and configure backend state management ([f6e56b2](https://github.com/Isuruzenith/X-Link/commit/f6e56b2ec91f22570d0d9a3de4c1f7ac7b37d922))
* resolve UI loading black/white screen in production by adding safe UUID fallback ([cd13db2](https://github.com/Isuruzenith/X-Link/commit/cd13db212d9e4a1088dff32155eb218c254da2db))
* update default MTU migration to 1400 in store and bypass local proxy routing for Clash API latency checks ([9af3509](https://github.com/Isuruzenith/X-Link/commit/9af3509e57e21740468604946a840896277de73a))


### Performance Improvements

* defer fetchVersions and delay show_window to prevent black startup screen ([dd58d05](https://github.com/Isuruzenith/X-Link/commit/dd58d051eceebe97f5af8ab84193f285f709c3d5))

## [0.7.0](https://github.com/Isuruzenith/X-Link/compare/x-link-v0.6.0...x-link-v0.7.0) (2026-07-04)


### Features

* add sing-box configuration generator with TUN and proxy routing support ([97e719e](https://github.com/Isuruzenith/X-Link/commit/97e719edcd3d88e3c4fd90fc4ed3573d9bded069))
* implement configurable TUN settings and dynamic Sing-box configuration generation ([456353d](https://github.com/Isuruzenith/X-Link/commit/456353d9694a29bc11252db3495e547ba46819fa))
* implement settings management system with UI view and backend configuration support ([bad1080](https://github.com/Isuruzenith/X-Link/commit/bad1080f6aaee64e7ec0b36a5cd6e2ac5ecbd4e5))
* implement sing-box configuration generator and core routing management services ([34466b5](https://github.com/Isuruzenith/X-Link/commit/34466b57d9055eb7efa01f9185c9812cf8b39beb))
* migrate configuration schemas for sing-box v1.13.14 compatibility ([2989c56](https://github.com/Isuruzenith/X-Link/commit/2989c56de2577a0751f037adac050115ecdf9079))


### Bug Fixes

* remove unconditional rule-sets direct routing to allow user-defined routing rules ([b6c5993](https://github.com/Isuruzenith/X-Link/commit/b6c5993f1662657421def58bb5095db2edf9a01a))
* resolve ESLint errors and strong type nodeGeoCache ([15e11c5](https://github.com/Isuruzenith/X-Link/commit/15e11c5793db8cee8cf435fcac1e7b625250f524))
* route DNS queries via port 53 hijack rule to prevent direct bypass ([8eeded2](https://github.com/Isuruzenith/X-Link/commit/8eeded2a554fa1fd03a99ed3327937f590ef91cc))

## [0.6.0](https://github.com/Isuruzenith/X-Link/compare/x-link-v0.5.0...x-link-v0.6.0) (2026-07-04)


### Features

* implement full application core with system management, configuration, routing, and node editing capabilities ([6cef87e](https://github.com/Isuruzenith/X-Link/commit/6cef87ed345a49c6f118d91b2e08020b547a65b0))

## [0.5.0](https://github.com/Isuruzenith/X-Link/compare/x-link-v0.4.0...x-link-v0.5.0) (2026-07-02)


### Features

* add CI and release GitHub Actions workflows ([1941e2f](https://github.com/Isuruzenith/X-Link/commit/1941e2f535c6731300b101993bb12e3a161de5d4))
* add CI/CD release workflow and system management commands for autostart, port checks, and versioning ([36aa8b9](https://github.com/Isuruzenith/X-Link/commit/36aa8b9f71cb7beffa9078b536a0c100d8caa193))
* add support for parsing raw proxy URI schemes including Shadowsocks and VMess ([b7ec544](https://github.com/Isuruzenith/X-Link/commit/b7ec544e4ef7e57a77217118f4e701a933d3ca99))
* implement node editor store and UI views for proxy configuration management ([f4bde2f](https://github.com/Isuruzenith/X-Link/commit/f4bde2fd596f4676c07cf085121820d84b47faf7))
* implement system utility commands for cross-platform autostart, elevation, and environment checks with CI integration ([bc8ea13](https://github.com/Isuruzenith/X-Link/commit/bc8ea1359b218c5a71eeec18c0a18f5e69b4283f))


### Bug Fixes

* **ci/frontend:** fix rust format check, upgrade node, and clean up connections view warnings ([fe5312b](https://github.com/Isuruzenith/X-Link/commit/fe5312b8ade69aae605978cc502c8ba54475161f))
* **tauri:** replace resource_dir with resolve for path resolver compatibility ([e5e2654](https://github.com/Isuruzenith/X-Link/commit/e5e265481bd8505bc21cae488e2611bb3c0ae732))
* **tauri:** use resolve with BaseDirectory::Resource for path resolver compatibility ([7ad43cd](https://github.com/Isuruzenith/X-Link/commit/7ad43cd910dea518314b820ebb69da8cbf635a86))

## [0.4.0](https://github.com/Isuruzenith/X-Link/compare/x-link-v0.3.0...x-link-v0.4.0) (2026-07-01)


### Features

* initialize core backend architecture with state management, command modules, and configuration generation logic ([7a79ea8](https://github.com/Isuruzenith/X-Link/commit/7a79ea8b42f254b52d92a5662fd0b7cf747a359b))
* initialize system proxy management, autostart functionality, and CI pipeline for Tauri project ([7e3ddeb](https://github.com/Isuruzenith/X-Link/commit/7e3ddeb6154aefdf481abde60ce13dc5f44d388b))

## [0.3.0](https://github.com/Isuruzenith/X-Link/compare/x-link-v0.2.0...x-link-v0.3.0) (2026-07-01)


### Features

* add ToastContainer component for displaying global notifications ([5864940](https://github.com/Isuruzenith/X-Link/commit/5864940a45b9108ac147fe503d7b925e6a05208d))
* create LogsView component with filtering, searching, and syntax-highlighted log display ([243514c](https://github.com/Isuruzenith/X-Link/commit/243514cca73948c05a8cfbae9e15fa1c1c7dd723))
* implement backend command for importing and managing sing-box configurations and add node editor store for UI state management ([b0e96b6](https://github.com/Isuruzenith/X-Link/commit/b0e96b64107649701dec46e955af749c2dabea2c))
* implement backend state management, proxy control commands, and frontend connection store ([236a590](https://github.com/Isuruzenith/X-Link/commit/236a5904d08d05cc33d6b0b3be6b48f4a0d2f681))
* implement ConfigView UI with profile management, node selection, and latency monitoring components ([2204913](https://github.com/Isuruzenith/X-Link/commit/2204913662673fa9a14246fc9c9a37f3049049bc))
* implement core proxy infrastructure and initial UI views for logs, connections, and dashboard ([3c85f38](https://github.com/Isuruzenith/X-Link/commit/3c85f389b0426d722a18dbbe3637ef7a0c70f7ff))
* implement cross-platform system utility commands and connection store for managing sing-box and app lifecycle. ([6fc8718](https://github.com/Isuruzenith/X-Link/commit/6fc8718a396824e2dcc8b2fb79de8ea0f7b7a7b4))
* implement log management store and real-time visualization interface with filtering and autoscroll ([989bfc6](https://github.com/Isuruzenith/X-Link/commit/989bfc6aa8880bf1e741bb822a0a4c7b51a3a7bc))
* implement multi-profile management system with configuration import and node switching support ([1ae1044](https://github.com/Isuruzenith/X-Link/commit/1ae1044c84b5c923c961c8ee1c78f58f40148cf6))
* implement multi-profile management system with persistent storage, node latency testing, and configuration import capabilities ([12f2822](https://github.com/Isuruzenith/X-Link/commit/12f2822648d59943173fbd8c9e814bd4f8ccf70b))
* implement proxy auto-connection, system tray management, and traffic monitoring orchestration ([96cbe00](https://github.com/Isuruzenith/X-Link/commit/96cbe00ed6c75bbcd2902c231d923eba730a14f8))
* implement sing-box proxy sidecar lifecycle management and configuration health checks ([eacc833](https://github.com/Isuruzenith/X-Link/commit/eacc8338a26cd50d2f416df56f360719164bc7c1))
* implement system tray menu with proxy control and server switching support ([a1bec58](https://github.com/Isuruzenith/X-Link/commit/a1bec58bebac64b0e1080af6f849a523482cac4a))
* implement toast notification system and integrate connection state feedback ([5be9b3a](https://github.com/Isuruzenith/X-Link/commit/5be9b3a3e59ebc39694bd6f0ba8c5b4600f21c98))
* move wintun.dll resource to separate tauri.windows.conf.json ([1795d4f](https://github.com/Isuruzenith/X-Link/commit/1795d4fc94191d79f18e9453580d8b340bcb1c6b))

## [0.2.0](https://github.com/Isuruzenith/X-Link/compare/x-link-v0.1.0...x-link-v0.2.0) (2026-07-01)


### Features

* add GitHub Actions release workflow and update project documentation ([6d799e7](https://github.com/Isuruzenith/X-Link/commit/6d799e7d4d9f4abc0cac0fd3c268349bb94baec6))
* add initial project setup with styles, main entry, and store management ([60bc2e3](https://github.com/Isuruzenith/X-Link/commit/60bc2e34f17cf777cbd32f97c9957f5c9404f02b))
* add NavRail component and implement global design system styles ([d51a3c1](https://github.com/Isuruzenith/X-Link/commit/d51a3c106682fa61efacacbe4f5cf037657bd3ae))
* add ProfilesView and SettingsView components for subscription management ([2c3211a](https://github.com/Isuruzenith/X-Link/commit/2c3211ad434fea933550b0704ed9ca16b4ae0499))
* add ProfilesView component for managing subscription profiles and server nodes ([5fab2ed](https://github.com/Isuruzenith/X-Link/commit/5fab2ede3f67657ccdb6f41750eaac545151a088))
* add raw URI parser adapter for VMess and Shadowsocks protocols ([7ed08c0](https://github.com/Isuruzenith/X-Link/commit/7ed08c024857148dd8bdfe6352a5604de857bc79))
* add release configuration files for version management ([3b00c99](https://github.com/Isuruzenith/X-Link/commit/3b00c99f5424c2f0c88bfddde8c84c512a9d107b))
* add Shadowsocks and VMess URI parsing support and system command module ([3e69a94](https://github.com/Isuruzenith/X-Link/commit/3e69a94f1e764f51ca458ad64b7226b84bd804bd))
* add sing-box config generation, process state management, tray icon integration, and proxy control commands ([ff50637](https://github.com/Isuruzenith/X-Link/commit/ff50637902010c6d897cfa906f7ddb9152b76b5a))
* add TrafficChart component, configure CI release workflow, update project version, and adjust linting rules ([a555f50](https://github.com/Isuruzenith/X-Link/commit/a555f508d9066ead076dc44e3657ede20255ea8b))
* implement comprehensive settings view and backend proxy command handlers ([2dfeefb](https://github.com/Isuruzenith/X-Link/commit/2dfeefb584a444a1a0946cf5dd791a1631a6703c))
* implement core application shell and settings configuration interface ([b3065b7](https://github.com/Isuruzenith/X-Link/commit/b3065b73329abe38dd25db3a34638ffd3ba62e7e))
* implement core application structure and state management for sing-box integration ([6f928a2](https://github.com/Isuruzenith/X-Link/commit/6f928a265997c62d611b04b66407df104e3d9462))
* implement DnsView and add application assets and icons ([78a0cd3](https://github.com/Isuruzenith/X-Link/commit/78a0cd3cd129693b783811af05feb7c3a4011f45))
* implement main application entry point and add initial project configuration ([a2185c1](https://github.com/Isuruzenith/X-Link/commit/a2185c1758e7c1210ce30ec1b16a01b40135c009))
* implement main application entry point and modular view components for X-Link ([0e19f7c](https://github.com/Isuruzenith/X-Link/commit/0e19f7cd068f3ce36a409c3e139f6561306caa33))
* implement navigation rail and core state management in App shell ([785d31f](https://github.com/Isuruzenith/X-Link/commit/785d31fe4e443b6f728953de33b5536376f7256c))
* implement NodeEditor component and UI foundation for node management ([7b08362](https://github.com/Isuruzenith/X-Link/commit/7b08362764286ddbc988ecb4b272dddffb35494a))
* implement persistent configuration store and initial project structure for X-Link ([bb64e65](https://github.com/Isuruzenith/X-Link/commit/bb64e65ca90deedbf1d69e42bca5d1061246b191))
* implement ProfilesView component and integrate with App state for subscription management ([60fce42](https://github.com/Isuruzenith/X-Link/commit/60fce428ec9c50fa8000baa665a3ddf65290b997))
* implement proxy command handlers and configuration management for TUN/sniffing settings ([805e1da](https://github.com/Isuruzenith/X-Link/commit/805e1da33510360f407d66155690452e6b26ce60))
* implement proxy command handlers for configuration, status, and traffic telemetry ([8bd5f24](https://github.com/Isuruzenith/X-Link/commit/8bd5f24af59509443746c8734437bda429f5807d))
* implement proxy management commands and configuration generation logic for sing-box integration ([6964760](https://github.com/Isuruzenith/X-Link/commit/69647605e73ce87f9adb4164b1520e617d28269d))
* implement proxy management commands and configuration hot-patching for X-Link backend ([7da5e61](https://github.com/Isuruzenith/X-Link/commit/7da5e61a487b2b26710f14fef7ee7c16b195f162))
* implement proxy management commands and dynamic configuration generation for sing-box ([ff07bd2](https://github.com/Isuruzenith/X-Link/commit/ff07bd24437009e82e4c911b5c62481e0f3ef17f))
* implement proxy management commands and dynamic configuration generation for sing-box integration ([7ffd322](https://github.com/Isuruzenith/X-Link/commit/7ffd322fea5acc833742b6eafea9ac9d0d9985bb))
* implement proxy management commands and dynamic configuration generation for sing-box integration ([9016b6b](https://github.com/Isuruzenith/X-Link/commit/9016b6bee6407a08f675781335a35191ed9c2b2f))
* implement proxy management commands and infrastructure for profile handling and configuration patching ([235eb3a](https://github.com/Isuruzenith/X-Link/commit/235eb3ae55a5f6b2cee104e9a5a027c501659772))
* implement proxy management commands and shared application state for background process orchestration ([97dbb09](https://github.com/Isuruzenith/X-Link/commit/97dbb09c2b735b2214b481a3c420ee0ca8572bfc))
* implement proxy state management and core control commands for sing-box service orchestration ([4df6e0d](https://github.com/Isuruzenith/X-Link/commit/4df6e0dbd19843840ac063415dc2c73ca0ac9eff))
* implement proxy status, traffic stats, and dynamic configuration generation commands ([d7deabc](https://github.com/Isuruzenith/X-Link/commit/d7deabca82db0997b8b263c4982ec50aee4c2824))
* implement release management workflow and setup sidecar binaries ([47b6f8d](https://github.com/Isuruzenith/X-Link/commit/47b6f8df535b4d339f8a0af67b5c4d39c4d54ac9))
* implement sing-box config generation and proxy state management commands ([df506e5](https://github.com/Isuruzenith/X-Link/commit/df506e51214211652310eaaa48080fb442c58d8a))
* implement sing-box config generation with TUN mode support and command handlers ([ff0767b](https://github.com/Isuruzenith/X-Link/commit/ff0767b66a27db5cee57c810c49e03fb2101e8af))
* implement sing-box config generator and subscription import command with support for system/tun modes ([1626f42](https://github.com/Isuruzenith/X-Link/commit/1626f425f2e62fe37f0ff58a08383cb8ebcc1079))
* implement sing-box configuration generator and core proxy command orchestration ([9e2af9e](https://github.com/Isuruzenith/X-Link/commit/9e2af9e4f59ff3a7f403908cc32da00148ccf84a))
* implement Sing-box configuration generator with TUN and SNI support ([52ccdba](https://github.com/Isuruzenith/X-Link/commit/52ccdbaad2129b2e0eb1cac1469a411c9fed24f6))
* implement system command layer and proxy state management for sing-box integration ([f567509](https://github.com/Isuruzenith/X-Link/commit/f567509fe6a371782d1d7eeb8af0d0f81bf78e5d))
* initialize core application shell, profile management, and configuration adapters ([98cdba8](https://github.com/Isuruzenith/X-Link/commit/98cdba8698fe2fb6e2b02432d46a19dede8fa216))
* initialize main App component with state management and core module navigation ([cf6915d](https://github.com/Isuruzenith/X-Link/commit/cf6915d407356df5f80075e220b37c2ca7fb7c88))
* initialize main App component with state management and core module structure ([1be56b1](https://github.com/Isuruzenith/X-Link/commit/1be56b1e07278b0e07c43d0f04f86dec0c91dc3c))
* initialize main application shell with navigation rail and core state management ([310d1e7](https://github.com/Isuruzenith/X-Link/commit/310d1e703521ddfc733121ce9dc1134992a03c04))
* initialize Tauri application structure with App UI components and proxy command bridge ([a455555](https://github.com/Isuruzenith/X-Link/commit/a4555557dc0baf8f07999f7503ad1c9fcac97e11))
* reinitialize project structure with core UI dashboard and backend foundation ([611bc80](https://github.com/Isuruzenith/X-Link/commit/611bc808cd4a2638987c290a88a448257748868d))
* update README with enhanced features and visuals, add new dependencies, and improve project structure ([aa3c961](https://github.com/Isuruzenith/X-Link/commit/aa3c96189339614b1d0cb1b022567b63d0d0c5ec))
* update Tauri project structure with cross-platform configuration and core application shell ([4cb5025](https://github.com/Isuruzenith/X-Link/commit/4cb50259de56e568b7393f0f94a2a688efad3d5b))


### Bug Fixes

* remove obsolete documentation files and update package dependencies ([aa3c961](https://github.com/Isuruzenith/X-Link/commit/aa3c96189339614b1d0cb1b022567b63d0d0c5ec))
* update authorship in Cargo.toml and remove obsolete binary files ([71d28d1](https://github.com/Isuruzenith/X-Link/commit/71d28d10ddf93108d55b728ea857524e559186a6))
