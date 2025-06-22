# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/fooker/pullomatic/compare/v0.1.0...v0.1.1) - 2025-06-22

### Added

- Improve nix package and module
- Allow to load secrets from files
- Refactor to be async
- Use tracing/logging library
- Use generic error lib
- Parse args using clap

### Fixed

- fixup! chore(release): Add release workflow
- *(deps)* Replace rust-crypto with RustCrypto
- *(deps)* update rust crate clap to v4.5.40 ([#14](https://github.com/fooker/pullomatic/pull/14))
- *(deps)* update rust crate clap to v4.5.39 ([#12](https://github.com/fooker/pullomatic/pull/12))
- *(deps)* update rust crate tokio to v1.45.1 ([#11](https://github.com/fooker/pullomatic/pull/11))
- *(deps)* update rust crate git2 to 0.20.0
- *(deps)* update rust crate git2 to 0.20
- fix dep issue
- fix syntax warning
- fixup! Added release uploads

### Other

- Remote pre-commit-hooks from code
- Add more metadata
- *(ci)* Fix clippy warnings
- *(ci)* Add formatting and clippy checks
- *(release)* Add release workflow
- *(deps)* Update nix and cargo dependencies
- fix clippy warnings
- *(ci)* Simplify build job
- *(deps)* Add renovate config
- add nix output to gitignore
- *(deps)* Update rust deps
- *(nix)* Add nix env
- Remove warnings
- Remove use of description
- More error handling
- use display
- cargo lock
- Version bump
- CI update
- Update deps
- Added release uploads
- Update issue templates
- Added build batch to readme
- Fix compile issues with rust 1.25.0
- Added Travis CI
- Create LICENSE
- Added webhooks and cleanup
- More work on readme
- Switched from TOML to YAML for config
- Make hook work
- Make interval more friendly
- Updated readme
- Renaming and cleanup
- Make authentication work
- Better error handling for config loading
- Rename to pullomatic
- Add hook and rework update flow
- Added initial readme
- Even more webhook
- More webhook impl
- CTRL-C and webhooks
- Implement queuing
- Initial import
