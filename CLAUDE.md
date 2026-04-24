# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Communication

Always use `/caveman` mode. Active every response. Off only if user says "stop caveman" or "normal mode".

## Critical constraints

**`core` and `embedded` are `#![no_std]`.**  Never introduce `std` into either crate. Concretely:
- Use `alloc::string::String`, `alloc::vec::Vec`, `alloc::boxed::Box`, `alloc::collections::BTreeMap` — not their `std::` equivalents.
- Use `core::fmt`, `core::f64::consts`, etc. — not `std::fmt`.
- `f64` methods that call platform libm (`.sin()`, `.fract()`, `.sqrt()`, …) must go through `libm::` crate functions instead, so the code links on bare-metal targets without a platform math library.
- No `println!`, `eprintln!`, `std::io`, threads, or anything else that pulls in `std`.

## Commands

```sh
# Build
cargo build                               # dev build (defaults to gui)
cargo build -p cli -p gui                 # build both desktop crates
cargo build -p cli --profile release-cli  # size-optimised CLI release
cargo build -p gui --release              # speed-optimised GUI release

# Test — always pass --lib for core (doctests fail due to crate name shadowing)
cargo test -p core --lib                  # all 28 CAS engine tests
cargo test -p core --lib <test_name>      # single test by name

# Run
cargo run                  # gui (workspace default)
cargo run -p cli           # REPL
cargo run -p gui           # egui desktop app
```

## Architecture

```
crates/
  core/      — #![no_std] CAS engine (shared by all targets)
  cli/       — std REPL binary
  gui/       — eframe/egui desktop app
  embedded/  — #![no_std] #![no_main] bare-metal target
```

### `core` — CAS engine

The pipeline is **parse → simplify → eval**.

| Module | Role |
|---|---|
| `rational` | Exact `i64`-based rational arithmetic. `checked_pow_int` guards overflow. |
| `expr` | `Expr` AST — `Rat`, `Float`, `Const`, `Var`, `Neg`, `Add(Vec)`, `Mul(Vec)`, `Pow`, `Call`. `Add`/`Mul` are n-ary, not binary trees. |
| `lexer` | Tokenises `&str`. Handles Unicode operators (`×`, `÷`, `·`) and scientific notation. |
| `parser` | Recursive descent. `/` becomes `a * b^(-1)`. `^` is right-associative. `!` is postfix factorial. |
| `simplify` | Bottom-up: recurse into children, then apply node-level rules. Collects like terms in `Add` (coefficient × symbolic part) and like bases in `Mul` (sum exponents). |
| `eval` | Numerical `f64` evaluation with a `BTreeMap<String, f64>` variable context. |

### `cli`

Thin wrapper over the core pipeline. Variable assignment (`x = expr`) stores results in `Context`. Falls back to displaying the simplified symbolic `Expr` when a variable is undefined.

### `gui`

eframe 0.34. The `App` trait requires `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame)` — not the old `update`. `Context` is held directly in `App` (can't `#[derive(Default)]` for it — orphan rule blocks implementing foreign traits on foreign types).

## Release profiles (root `Cargo.toml`)

| Profile | `opt-level` | extras | use for |
|---|---|---|---|
| `release` | `3` | LTO, 1 codegen unit, strip | GUI |
| `release-cli` | `"z"` | + `panic = "abort"` | CLI |
