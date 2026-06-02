# Chiba Level-1R

Rust reference compiler for Chiba level-1 bootstrap.

This crate is intentionally test-first. It starts with a tiny expression core, visible alpha/typed/usage/CPS reports, and one-pass CPS invariants, then grows toward the full nanopass compiler described by `../level-1r.md`, `../TODO.level1-b.md`, and `../TODO.longterm.md`.

Current baseline:

- source expression model
- alpha conversion with stable binder ids and undefined-name diagnostics
- typed expression skeleton
- binder-id usage facts
- one-pass CBV CPS with meta-continuations
- delimiter-driven Cont1/ContN facts
- closure capture facts keyed by resolved binder id
- target-neutral CoreIR skeleton
- chibalex maximal-munch lexer baseline
- chibacc token parser baseline
- chibacc Pratt expression parser baseline
- visual report across source / alpha / typed / usage / cps

Run:

```sh
cargo test --manifest-path level-1r/Cargo.toml
cargo run --manifest-path level-1r/Cargo.toml
```
