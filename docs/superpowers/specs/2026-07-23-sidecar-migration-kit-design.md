# Sidecar Native-Module Migration Kit — Design

**Date:** 2026-07-23
**Status:** Spec — ready to implement (BACKLOG §5.3)
**Depends on:** `wrapswap-core` (parsers, matrix, scaffolder)

## Problem

Electron apps' native Node modules (N-API/node-gyp) have no Tauri equivalent — Tauri has no
Node runtime, so each native module must be re-authored as a Rust command or run as an external
**sidecar** process. The documented migration cost for native-module-heavy apps is the single
biggest pain point (Brief §5.3). This kit automates the sidecar path: detect native-module usage
and scaffold a runnable sidecar skeleton + the Tauri wiring to talk to it.

## Scope

- **In:** detect native modules from an Electron project; generate (a) a sidecar binary skeleton
  (Rust or a documented Node/other-lang stub) that speaks a simple line-delimited JSON protocol
  over stdio, (b) the Tauri-side `tauri-plugin-shell` sidecar config + a typed Rust client, and
  (c) a migration checklist per module.
- **Out:** actually reimplementing any module's logic; binary analysis of `.node` files.

## Detection

Extend `wrapswap-core::parse` with `detect_native_modules(package_json, node_modules_dir?) ->
Vec<NativeModule>`:
- Scan `dependencies`/`optionalDependencies` for packages containing a `binding.gyp`, a
  `prebuilds/` dir, or known N-API markers (`node-addon-api`, `node-gyp-build`, `bindings`,
  `ffi-napi`, `napi`).
- Each `NativeModule { name, detection_reason, has_prebuilds }`.

## Generated artifacts (per project)

1. `sidecar/Cargo.toml` + `sidecar/src/main.rs` — a stdio loop reading one JSON request per line
   (`{"id":N,"module":"...","method":"...","args":[...]}`) and writing one JSON response per line
   (`{"id":N,"ok":true,"result":...}` / `{"id":N,"ok":false,"error":"..."}`). One `todo!()`-bodied
   handler stub per detected module+method-placeholder, with the detection reason in a comment.
2. `src-tauri` snippet — `tauri-plugin-shell` `externalBin` config for the sidecar, plus a Rust
   `SidecarClient` (spawn, request/response correlation by `id`, timeout) as a ready module.
3. `MIGRATION.md` — one row per module: name, why flagged, has-prebuilds, and the decision
   (reimplement-in-Rust vs keep-as-sidecar), left for the developer.

## CLI surface (new subcommand on `wrap-swap`)

`wrap-swap sidecar --electron-pkg package.json [--node-modules ./node_modules] --out-dir sidecar-out`
→ writes the three artifacts; prints the module count and the honesty note that all handlers are
stubs (no logic is ported).

## Testing

- `detect_native_modules` unit tests over fixture `package.json` (+ a fake `binding.gyp` marker).
- Golden/`syn`-parse tests that the generated `main.rs` and `SidecarClient` are valid Rust.
- Integration: `sidecar` subcommand writes all three files for a fixture project.

## Honesty invariant

Every generated handler is an explicit `todo!()` stub with its detection reason; the kit never
claims a module is "migrated", only scaffolded. `MIGRATION.md` makes the remaining work explicit.
