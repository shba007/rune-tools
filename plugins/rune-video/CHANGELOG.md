# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

- - -

## [rune-video-v0.3.0](https://github.com/shba007/rune-tools/compare/rune-video-v0.2.0..rune-video-v0.3.0) - 2026-10-21

#### 🚀 Features
- (**rune-ssh**) add SSH and SFTP operations with WASM support - ([9cc5df9](https://github.com/shba007/rune-tools/commit/9cc5df9a642fe884fa1c88316052ba4caa67711e)) - Shirsendu Bairagi
- enhance tool rules validation and annotations - ([482c069](https://github.com/shba007/rune-tools/commit/482c069a1560b505f75bc9479c74818ac390b954)) - Shirsendu Bairagi
- implement tool safety rules, A2UI blueprints, and handoff report - ([e40d5f1](https://github.com/shba007/rune-tools/commit/e40d5f1db8cfce55afa331d33e0abfcf84dce097)) - Shirsendu Bairagi

- - -

## [rune-video-v0.2.0](https://github.com/shba007/rune-tools/compare/rune-video-v0.1.2..rune-video-v0.2.0) - 2026-09-30

#### 🚀 Features
- (**rune-browser**) add BrowserBinaryConfig and PlatformConfig for binary management - ([a18e534](https://github.com/shba007/rune-tools/commit/a18e5346481baf19c557a59a9f2b598e624395c5)) - Shirsendu Bairagi
- add initial implementation of rune-figma plugin with operations and types - ([6cb7671](https://github.com/shba007/rune-tools/commit/6cb76715e450a647b53ef0ca6856a1f2a7ed6a55)) - Shirsendu Bairagi
- (**rune-fetch**) update version to 0.3.0, enhance fetching capabilities, and add resource/prompt definitions - ([fe5f017](https://github.com/shba007/rune-tools/commit/fe5f017400faa404cd5f009049b5b7979b2ea27d)) - Shirsendu Bairagi

#### 🚜 Refactoring
- dependencies and remove obsolete plugin - ([fefd5f0](https://github.com/shba007/rune-tools/commit/fefd5f01a3123d2176c5e672d59ab9c835662d2a)) - Shirsendu Bairagi

#### ⚙️ Miscellaneous
- update dependencies and improve plugin descriptions for rune-audio and rune-video - ([bea4cfc](https://github.com/shba007/rune-tools/commit/bea4cfc53856bfc016fb0696d5e379692cba9e05)) - Shirsendu Bairagi
- bump versions and update plugin metadata for rune-memory, rune-sequential-thinking, rune-slides, rune-time, and rune-video - ([b05a3d5](https://github.com/shba007/rune-tools/commit/b05a3d53e4bccb9bed802ad767ca3751f8a43aa3)) - Shirsendu Bairagi

- - -

## [rune-video-v0.1.2](https://github.com/shba007/rune-tools/compare/rune-video-v0.1.1..rune-video-v0.1.2) - 2026-09-17

#### 🚀 Features
- (**rune-audio**) update description and add README with detailed use cases for audio extraction and music downloading tools - ([079e873](https://github.com/shba007/rune-tools/commit/079e873de4a44d050836046d8e0543b14d40ad62)) - Shirsendu Bairagi
- add MCP configuration files for multiple plugins including audio, email, fetch, filesystem, git, image, memory, sequentialthinking, time, and video - ([44a9011](https://github.com/shba007/rune-tools/commit/44a90113e3749989d4247727f5fde70a2c8171b6)) - Shirsendu Bairagi
- revamp readme and plugin documentation - ([4f88a2b](https://github.com/shba007/rune-tools/commit/4f88a2ba07d2d35b66655ce15ab44cba0b919941)) - Shirsendu Bairagi
- add multiple new plugins with documentation and capabilities - ([b4f45bb](https://github.com/shba007/rune-tools/commit/b4f45bbb370912831aa5f473ef0e443970fb7c49)) - Shirsendu Bairagi

#### 🚜 Refactoring
- paths and update authors in plugin configurations - ([3c795fd](https://github.com/shba007/rune-tools/commit/3c795fd3ee232612a4e4cd642b2be3f71a0e444c)) - Shirsendu Bairagi

#### ⚙️ Miscellaneous
- update plugin versions for * - ([6f02ac2](https://github.com/shba007/rune-tools/commit/6f02ac20a8e96d2edd3f53fd9852c4d83c7fa1a1)) - Shirsendu Bairagi

- - -

## [rune-video-v0.1.1](https://github.com/shba007/rune-tools/compare/rune-video-v0.1.0..rune-video-v0.1.1) - 2026-09-02

#### 🚀 Features
- introduce xtask for streamlined plugin builds/ tests updated rune-filesystem for image as a image return - ([09f7421](https://github.com/shba007/rune-tools/commit/09f742135575c702a43010b5bb0439446a71f6cf)) - Shirsendu Bairagi

- - -

## rune-video-v0.1.0 - 2026-09-02

#### 🚀 Features
- project structure with cargo configuration and plugin implementations - ([eece334](https://github.com/shba007/rune-tools/commit/eece33483266bfb48760196a99b4d9663e65bce6)) - Shirsendu Bairagi
- add rune-time plugin for timezone conversion and enhance rune-fs plugin - ([6de898b](https://github.com/shba007/rune-tools/commit/6de898b06bc5eaa727e4987b6d62a616e5795996)) - Shirsendu Bairagi
- implement a test suite for WASM plugins to validate filesystem operations. - ([17b9900](https://github.com/shba007/rune-tools/commit/17b99001e554b9aa5a692dd17cf8a22b38c82a09)) - Shirsendu Bairagi
- Introduce rune-sidecar crate and enhance rune-fetch plugin - ([787b64b](https://github.com/shba007/rune-tools/commit/787b64b176edb48deac4cfc2efe7c0b0696644d3)) - Shirsendu Bairagi
- (**rune-git**) add native sidecar binary with stdio JSON-RPC protocol - ([e871293](https://github.com/shba007/rune-tools/commit/e8712937cc49f9465af863c63737a760d8f341db)) - Shirsendu Bairagi
- validate 'url' parameter in execute_tool function and update test for title assertion - ([6281ede](https://github.com/shba007/rune-tools/commit/6281ede5369b962f5341b2cbc1d857228a69c132)) - Shirsendu Bairagi

#### 🚜 Refactoring
- replace rune-media plugin with specialized rune-video plugin - ([b2c36e5](https://github.com/shba007/rune-tools/commit/b2c36e5b97377e13b5b3059dda37ea396585a895)) - Shirsendu Bairagi
- reorder use statements for consistency in native sidecar files - ([460aec0](https://github.com/shba007/rune-tools/commit/460aec0325a8b795e10d8686785f85b6bc4f3c50)) - Shirsendu Bairagi
- simplify conditional checks and improve readability in various operations - ([a671986](https://github.com/shba007/rune-tools/commit/a67198681b37e4be80f9a144882c325a79303134)) - Shirsendu Bairagi
- rename tests to indicate live execution and skip in CI environment - ([f28e0e8](https://github.com/shba007/rune-tools/commit/f28e0e8f6c52c697e2174d435bc8cf009ad231b3)) - Shirsendu Bairagi

- - -

Changelog generated by [cocogitto](https://github.com/cocogitto/cocogitto).
