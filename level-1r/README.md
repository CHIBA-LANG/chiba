# Chiba Level-1R

Rust reference compiler for Chiba level-1 bootstrap.

This crate is intentionally test-first. It starts with a tiny expression core, visible typed/usage/CPS reports, and one-pass CPS invariants, then grows toward the full nanopass compiler described by `../level-1r.md`, `../TODO.level1-b.md`, and `../TODO.longterm.md`.

Current baseline:

- source expression model
- typed expression skeleton
- usage facts
- one-pass CBV CPS with meta-continuations
- visual report across source / typed / usage / cps

Run:

```sh
cargo test --manifest-path level-1r/Cargo.toml
cargo run --manifest-path level-1r/Cargo.toml
```
