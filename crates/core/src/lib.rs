#![no_std]
#[macro_use]
extern crate alloc;

// ── submodule groups ──────────────────────────────────────────────────────────
mod algebra;
mod parse;
mod runtime;
#[cfg(feature = "scripting")]
pub mod scripting;
pub mod selftest;
#[doc(hidden)]
pub mod tests;
mod types;

// ── flat re-exports (keeps `use crate::X` working inside every module) ────────
pub use algebra::diff;
pub use algebra::integrate;
pub use algebra::series;
pub use algebra::simplify;
pub use algebra::solve;
pub use algebra::subst;
pub use parse::lexer;
pub use parse::parser;
pub use parse::syntax;
pub use runtime::env;
pub use runtime::eval;
pub use runtime::matrix;
pub use types::error;
pub use types::expr;
pub use types::rational;

// ── public API ────────────────────────────────────────────────────────────────
pub use diff::diff;
pub use env::{Env, UserFn};
pub use error::CalcError;
pub use eval::{eval, eval_env, Context};
pub use expr::Expr;
pub use parser::{parse, parse_statement, Statement};
#[cfg(feature = "scripting")]
pub use scripting::{CompiledScript, ScriptRuntime, ScriptScope};
pub use simplify::{simplify, simplify_with_env};
