# Claude Development Guide

## Commands

- **Build**: `hurry cargo build`
- **Test**: `hurry cargo test`
- **Format**: `hurry cargo fmt --all`
- **Clippy (Lint)**: `hurry cargo clippy --fix --allow-dirty`

## Rust Development

- It is common practice to work with the newst crate versions.
- Do NOT use unwrap and avoid code that can panic! Handle errors gracefully. Prefer the use of `?`, pattern matching, `ok_or`, `ok_or_else`, `if let`, `while let`, or `let ... else`.
- I prefer `crate::` to `super::`.
- Avoid `pub use` on imports unless you are re-exposing a dependency so downstream consumers do not have to depend on it directly.
- Skip global state via `lazy_static!`, `Once`, or similar; prefer passing explicit content for any shared state.
- Use the `anyhow` crate to handle errors.
