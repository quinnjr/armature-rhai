# Changelog — `armature-rhai`

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Earlier changes are recorded in the workspace [`CHANGELOG.md`](../CHANGELOG.md).

## [Unreleased]

## [0.6.0] - 2026-10-17

### Security

- `ScriptRouter` no longer returns script error details to clients: an unhandled script error yields a plain `500 Internal Server Error` body and the error is logged server-side.
- The request span's `path` field records the path without its query string, so tokens or reset codes in the query no longer reach traces.
- Every `RhaiEngine` is hardened: the `eval` function is disabled and `import` is refused unless enabled with `RhaiEngineBuilder::allow_imports(true)`, which confines imports to `scripts_dir` (no absolute paths, `..`, or symlinks leading outside).

### Added

- `harden_engine(&mut rhai::Engine)` (disable `eval`, refuse imports) and `SandboxedModuleResolver` (imports confined to one directory) for engines built outside `RhaiEngine`.
- `EngineConfig::allow_imports` / `RhaiEngineBuilder::allow_imports`.

### Changed

- **Breaking:** scripts can no longer call `eval`, and `import` fails unless `allow_imports(true)` is set. Migration: call `.allow_imports(true)` and keep imported modules under `scripts_dir`; replace `eval` with real functions.
- **Breaking:** `EngineConfig` gained the public field `allow_imports`; struct literals need `allow_imports: false` (or `..Default::default()`).

## [0.5.0] - 2026-09-15

### Changed

- **Breaking:** requires `armature-core` 0.10 (was `0.9`); its types appear in this crate's API, so the requirement change is breaking here and the minor moves. Part of the `armature-core` 0.10 release train.
- Dependencies bumped to their latest releases: `rhai` 1.25 → 1.26, `tokio` 1.52 → 1.53, `tokio` 1.52 → 1.53.

## [0.4.0] - 2026-08-05

### Changed

- **Requires `armature-core` 0.9 (breaking).** The requirement moved `0.8` →
  `0.9`. `armature-core 0.9.0` itself moves `armature-h1` across a breaking
  0.x boundary; because `armature-core` types appear in this crate's own
  public API, the requirement change is breaking here too and the minor moves
  with it. Under Cargo's 0.x caret rules the 0.8 and 0.9 types are distinct
  and do not unify, so a consumer holding an `armature-core 0.8` type cannot
  pass it to this crate. Part of the `armature-core 0.9.0` release train; see
  `armature-core`'s CHANGELOG for the publish order.

## [0.3.1] - 2026-08-04

### Fixed

- Requirements on sibling armature crates name a minor instead of `0`. Under
  Cargo's 0.x rules `version = "0"` matches any release ever made, and edition
  2024 selects the MSRV-aware resolver, so a consumer declaring an older
  `rust-version` was handed the oldest version satisfying it — resolving
  `armature-core = "0"` on Rust 1.89 produced `armature-core 0.2.3` while an
  explicit `armature-core = "0.8"` elsewhere in the same graph pulled 0.8.2.
  Two copies of core, and a build failing on symbols the older one lacks. Each
  0.x minor in this family is a breaking change, so the requirement now names
  one. No API change.
