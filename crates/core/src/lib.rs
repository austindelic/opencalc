#![no_std]
#[macro_use]
extern crate alloc;

// ── submodule groups ──────────────────────────────────────────────────────────
mod types;
mod parse;
mod algebra;
mod runtime;

// ── flat re-exports (keeps `use crate::X` working inside every module) ────────
pub use types::error;
pub use types::rational;
pub use types::expr;
pub use parse::syntax;
pub use parse::lexer;
pub use parse::parser;
pub use algebra::subst;
pub use algebra::simplify;
pub use algebra::diff;
pub use algebra::integrate;
pub use algebra::series;
pub use algebra::solve;
pub use runtime::env;
pub use runtime::matrix;
pub use runtime::eval;

// ── public API ────────────────────────────────────────────────────────────────
pub use error::CalcError;
pub use expr::Expr;
pub use parser::{parse, parse_statement, Statement};
pub use simplify::simplify;
pub use eval::{eval, eval_env, Context};
pub use env::{Env, UserFn};
pub use diff::diff;
