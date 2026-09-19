# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- `oreos_runtime::install_defmt_timestamp!()` — wraps the defmt panic-handler +
  timestamp boilerplate every consumer crate previously had to hand-write.
- `oreos-runtime` now re-exports `defmt`, `embassy_time`, and `fugit` at its
  crate root, reachable as `Oreos::defmt`, `Oreos::embassy_time`,
  `Oreos::fugit` — see `docs/FRAMEWORK.md` §2.3.

### Changed

### Fixed

- `ordl generate`'s output now compiles cleanly against the current
  `oreos-runtime`/`oreos-macros` trait layer (verified end-to-end against a
  real consumer crate). Fixed: `Backend` template not matching the real
  `hal::Backend` trait shape, `#[backends]`/`#[backend]` attribute name
  mismatch, missing `Command`/middleware scaffolding, cross-module imports
  for kernel/middleware/device-component types, and `Kernel`/`Bus` struct
  visibility.
- Macro-generated code (and `install_defmt_timestamp!()`) no longer requires
  consumer crates to add `defmt`, `fugit`, or `embassy-time` as direct
  dependencies — bare extern-crate paths were replaced with paths through
  `oreos-runtime`'s re-export of those crates.

### Removed
