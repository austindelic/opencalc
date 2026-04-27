use crate::env::Env;
use crate::error::CalcError;
use crate::expr::{BuiltinFn, Constant, Expr};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};

// LCG PRNG — no-std safe, no external entropy required.
// Produces pseudo-random numbers; not cryptographically secure.
// Falls back to a 32-bit LCG on targets without 64-bit atomics (e.g. Cortex-M33).
#[cfg(target_has_atomic = "64")]
mod prng {
    use core::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(0x_dead_beef_cafe_babe);
    pub fn rand() -> f64 {
        loop {
            let old = SEED.load(Ordering::Relaxed);
            let new = old
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            if SEED
                .compare_exchange_weak(old, new, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return (new >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
            }
        }
    }
}

#[cfg(not(target_has_atomic = "64"))]
mod prng {
    use core::sync::atomic::{AtomicU32, Ordering};
    static SEED: AtomicU32 = AtomicU32::new(0xdead_beef);
    pub fn rand() -> f64 {
        loop {
            let old = SEED.load(Ordering::Relaxed);
            let new = old.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            if SEED
                .compare_exchange_weak(old, new, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return (new >> 8) as f64 * (1.0 / (1u32 << 24) as f64);
            }
        }
    }
}

fn lcg_rand() -> f64 {
    prng::rand()
}

/// Lightweight numeric context (f64 variables only).
/// Kept for backward compatibility; prefer `Env` for new code.
pub struct Context {
    vars: BTreeMap<String, f64>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            vars: BTreeMap::new(),
        }
    }
    pub fn set(&mut self, name: &str, value: f64) {
        self.vars.insert(name.to_string(), value);
    }
    pub fn get(&self, name: &str) -> Option<f64> {
        self.vars.get(name).copied()
    }
}

impl Default for Context {
    fn default() -> Self {
        Context::new()
    }
}

/// Numerically evaluate an expression with a simple f64 variable context.
pub fn eval(expr: &Expr, ctx: &Context) -> Result<f64, CalcError> {
    let env = ctx_to_env(ctx);
    eval_env(expr, &env)
}

fn ctx_to_env(ctx: &Context) -> Env {
    let mut env = Env::new();
    for (k, v) in &ctx.vars {
        env.set_var(k, Expr::Float(*v));
    }
    env
}

/// Numerically evaluate an expression using a full `Env`.
pub fn eval_env(expr: &Expr, env: &Env) -> Result<f64, CalcError> {
    match expr {
        Expr::Rat(r) => Ok(r.to_f64()),
        Expr::Float(v) => Ok(*v),
        Expr::Const(c) => Ok(match c {
            Constant::Pi => core::f64::consts::PI,
            Constant::E => core::f64::consts::E,
            Constant::Inf => f64::INFINITY,
            Constant::NegInf => f64::NEG_INFINITY,
            Constant::I => {
                return Err(CalcError::InvalidArgument(
                    "complex 'i' cannot be evaluated to f64".into(),
                ));
            }
        }),
        Expr::Var(name) => match env.get_var(name) {
            Some(e) => eval_env(e, env),
            None => Err(CalcError::UndefinedVariable(name.clone())),
        },
        Expr::Neg(inner) => Ok(-eval_env(inner, env)?),
        Expr::Add(terms) => {
            let mut acc = 0.0f64;
            for t in terms {
                acc += eval_env(t, env)?;
            }
            Ok(acc)
        }
        Expr::Mul(factors) => {
            let mut acc = 1.0f64;
            for f in factors {
                acc *= eval_env(f, env)?;
            }
            Ok(acc)
        }
        Expr::Pow(base, exp) => Ok(libm::pow(eval_env(base, env)?, eval_env(exp, env)?)),
        Expr::Call(f, args) => eval_call(f, args, env),
        Expr::FnCall(name, args) => {
            let (params, body) = {
                let uf = env.get_fn(name).ok_or_else(|| {
                    CalcError::UndefinedVariable(format!("function '{}' not defined", name))
                })?;
                (uf.params.clone(), uf.body.clone())
            };
            if args.len() != params.len() {
                return Err(CalcError::InvalidArgument(format!(
                    "'{}' expects {} args, got {}",
                    name,
                    params.len(),
                    args.len()
                )));
            }
            let mut call_env = env.clone();
            for (param, arg) in params.iter().zip(args.iter()) {
                let v = eval_env(arg, env)?;
                call_env.set_var(param, Expr::Float(v));
            }
            eval_env(&body, &call_env)
        }
        // Lists / matrices: evaluate each element
        Expr::List(items) => {
            // Return the first element or error if non-scalar needed
            if items.len() == 1 {
                return eval_env(&items[0], env);
            }
            Err(CalcError::InvalidArgument(
                "list cannot be evaluated as a scalar".into(),
            ))
        }
        Expr::Matrix(_) => Err(CalcError::InvalidArgument(
            "matrix cannot be evaluated as a scalar".into(),
        )),
        Expr::Equation(l, r) => {
            // Evaluate as boolean (1.0 if true, 0.0 if false)
            let lv = eval_env(l, env)?;
            let rv = eval_env(r, env)?;
            Ok(if (lv - rv).abs() < 1e-12 { 1.0 } else { 0.0 })
        }
    }
}

fn arg(args: &[Expr], i: usize, env: &Env) -> Result<f64, CalcError> {
    args.get(i)
        .ok_or_else(|| CalcError::InvalidArgument(format!("missing argument {}", i)))
        .and_then(|e| eval_env(e, env))
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let mut i = 3i64;
    while i * i <= n {
        if n % i == 0 {
            return false;
        }
        i += 2;
    }
    true
}

fn eval_call(f: &BuiltinFn, args: &[Expr], env: &Env) -> Result<f64, CalcError> {
    match f {
        BuiltinFn::Sin => Ok(libm::sin(arg(args, 0, env)?)),
        BuiltinFn::Cos => Ok(libm::cos(arg(args, 0, env)?)),
        BuiltinFn::Tan => Ok(libm::tan(arg(args, 0, env)?)),
        BuiltinFn::Asin => Ok(libm::asin(arg(args, 0, env)?)),
        BuiltinFn::Acos => Ok(libm::acos(arg(args, 0, env)?)),
        BuiltinFn::Atan => Ok(libm::atan(arg(args, 0, env)?)),
        BuiltinFn::Atan2 => Ok(libm::atan2(arg(args, 0, env)?, arg(args, 1, env)?)),
        BuiltinFn::Sinh => Ok(libm::sinh(arg(args, 0, env)?)),
        BuiltinFn::Cosh => Ok(libm::cosh(arg(args, 0, env)?)),
        BuiltinFn::Tanh => Ok(libm::tanh(arg(args, 0, env)?)),
        BuiltinFn::Asinh => Ok(libm::asinh(arg(args, 0, env)?)),
        BuiltinFn::Acosh => Ok(libm::acosh(arg(args, 0, env)?)),
        BuiltinFn::Atanh => Ok(libm::atanh(arg(args, 0, env)?)),
        BuiltinFn::Exp => Ok(libm::exp(arg(args, 0, env)?)),
        BuiltinFn::Ln => {
            let x = arg(args, 0, env)?;
            if x <= 0.0 {
                return Err(CalcError::InvalidArgument("ln requires x > 0".into()));
            }
            Ok(libm::log(x))
        }
        BuiltinFn::Log => {
            if args.len() == 2 {
                let base = arg(args, 0, env)?;
                let x = arg(args, 1, env)?;
                Ok(libm::log(x) / libm::log(base))
            } else {
                Ok(libm::log(arg(args, 0, env)?))
            }
        }
        BuiltinFn::Log2 => Ok(libm::log2(arg(args, 0, env)?)),
        BuiltinFn::Log10 => Ok(libm::log10(arg(args, 0, env)?)),
        BuiltinFn::Sqrt => {
            let x = arg(args, 0, env)?;
            if x < 0.0 {
                return Err(CalcError::InvalidArgument("sqrt requires x >= 0".into()));
            }
            Ok(libm::sqrt(x))
        }
        BuiltinFn::Cbrt => Ok(libm::cbrt(arg(args, 0, env)?)),
        BuiltinFn::Abs => Ok(libm::fabs(arg(args, 0, env)?)),
        BuiltinFn::Floor => Ok(libm::floor(arg(args, 0, env)?)),
        BuiltinFn::Ceil => Ok(libm::ceil(arg(args, 0, env)?)),
        BuiltinFn::Round => Ok(libm::round(arg(args, 0, env)?)),
        BuiltinFn::Sign => {
            let x = arg(args, 0, env)?;
            Ok(if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            })
        }
        BuiltinFn::Factorial => {
            let x = arg(args, 0, env)?;
            if x < 0.0 || x != libm::floor(x) {
                return Err(CalcError::InvalidArgument(
                    "factorial requires a non-negative integer".into(),
                ));
            }
            let n = x as u64;
            if n > 20 {
                return Err(CalcError::Overflow);
            }
            Ok((1..=n).product::<u64>() as f64)
        }
        BuiltinFn::Gcd => {
            let a = arg(args, 0, env)? as i64;
            let b = arg(args, 1, env)? as i64;
            Ok(gcd(a, b) as f64)
        }
        BuiltinFn::Lcm => {
            let a = arg(args, 0, env)? as i64;
            let b = arg(args, 1, env)? as i64;
            let g = gcd(a, b);
            if g == 0 {
                return Ok(0.0);
            }
            Ok((a / g * b).unsigned_abs() as f64)
        }
        BuiltinFn::Mod => {
            let a = arg(args, 0, env)?;
            let b = arg(args, 1, env)?;
            if b == 0.0 {
                return Err(CalcError::DivisionByZero);
            }
            Ok(libm::fmod(a, b))
        }
        BuiltinFn::Max => {
            let a = arg(args, 0, env)?;
            let b = arg(args, 1, env)?;
            Ok(if a >= b { a } else { b })
        }
        BuiltinFn::Min => {
            let a = arg(args, 0, env)?;
            let b = arg(args, 1, env)?;
            Ok(if a <= b { a } else { b })
        }
        BuiltinFn::IsPrime => {
            let n = arg(args, 0, env)? as i64;
            Ok(if is_prime(n) { 1.0 } else { 0.0 })
        }
        // Symbolic ops resolved during simplify; if we get here, evaluate numerically
        BuiltinFn::NDiff => {
            // ndiff(expr, var, point)
            if args.len() < 3 {
                return Err(CalcError::InvalidArgument("ndiff needs 3 args".into()));
            }
            let point = arg(args, 2, env)?;
            let h = 1e-7;
            let mut e = env.clone();
            let var = match &args[1] {
                Expr::Var(v) => v.clone(),
                _ => {
                    return Err(CalcError::InvalidArgument(
                        "ndiff: second arg must be a variable".into(),
                    ));
                }
            };
            e.set_var(&var, Expr::Float(point + h));
            let f1 = eval_env(&args[0], &e)?;
            e.set_var(&var, Expr::Float(point - h));
            let f2 = eval_env(&args[0], &e)?;
            Ok((f1 - f2) / (2.0 * h))
        }
        BuiltinFn::NIntegrate | BuiltinFn::Integral => {
            if args.len() < 4 {
                return Err(CalcError::InvalidArgument("integral needs 4 args".into()));
            }
            let var = match &args[1] {
                Expr::Var(v) => v.clone(),
                _ => {
                    return Err(CalcError::InvalidArgument(
                        "integral: second arg must be variable".into(),
                    ));
                }
            };
            let a = arg(args, 2, env)?;
            let b = arg(args, 3, env)?;
            crate::integrate::nintegrate(&args[0], &var, a, b, 1000)
        }
        BuiltinFn::Sum => {
            // sum(expr, var, from, to)
            if args.len() < 4 {
                return Err(CalcError::InvalidArgument("sum needs 4 args".into()));
            }
            let var = match &args[1] {
                Expr::Var(v) => v.clone(),
                _ => {
                    return Err(CalcError::InvalidArgument(
                        "sum: second arg must be variable".into(),
                    ));
                }
            };
            let from = arg(args, 2, env)? as i64;
            let to = arg(args, 3, env)? as i64;
            let mut acc = 0.0f64;
            let mut e = env.clone();
            for i in from..=to {
                e.set_var(&var, Expr::Float(i as f64));
                acc += eval_env(&args[0], &e)?;
            }
            Ok(acc)
        }
        BuiltinFn::Product => {
            if args.len() < 4 {
                return Err(CalcError::InvalidArgument("product needs 4 args".into()));
            }
            let var = match &args[1] {
                Expr::Var(v) => v.clone(),
                _ => {
                    return Err(CalcError::InvalidArgument(
                        "product: second arg must be variable".into(),
                    ));
                }
            };
            let from = arg(args, 2, env)? as i64;
            let to = arg(args, 3, env)? as i64;
            let mut acc = 1.0f64;
            let mut e = env.clone();
            for i in from..=to {
                e.set_var(&var, Expr::Float(i as f64));
                acc *= eval_env(&args[0], &e)?;
            }
            Ok(acc)
        }
        BuiltinFn::Len => match args.first() {
            Some(Expr::List(items)) => Ok(items.len() as f64),
            Some(Expr::Matrix(rows)) => Ok(rows.len() as f64),
            _ => Err(CalcError::InvalidArgument(
                "len expects a list or matrix".into(),
            )),
        },
        BuiltinFn::Norm => match args.first() {
            Some(Expr::List(items)) => {
                let mut sum = 0.0f64;
                for e in items {
                    let v = eval_env(e, env)?;
                    sum += v * v;
                }
                Ok(libm::sqrt(sum))
            }
            _ => Ok(libm::fabs(arg(args, 0, env)?)),
        },
        BuiltinFn::Re => Ok(arg(args, 0, env)?),
        BuiltinFn::Im => Ok(0.0),
        BuiltinFn::Conj => Ok(arg(args, 0, env)?),
        BuiltinFn::Arg => {
            let x = arg(args, 0, env)?;
            Ok(if x >= 0.0 { 0.0 } else { core::f64::consts::PI })
        }
        BuiltinFn::If => {
            let cond = arg(args, 0, env)?;
            if cond != 0.0 {
                arg(args, 1, env)
            } else {
                arg(args, 2, env)
            }
        }
        BuiltinFn::Random => {
            let r = lcg_rand();
            if args.len() == 2 {
                let lo = arg(args, 0, env)?;
                let hi = arg(args, 1, env)?;
                Ok(lo + r * (hi - lo))
            } else {
                Ok(r)
            }
        }
        BuiltinFn::Numerator => match args.first() {
            Some(Expr::Rat(r)) => Ok(r.numer as f64),
            _ => Ok(arg(args, 0, env)?),
        },
        BuiltinFn::Denominator => match args.first() {
            Some(Expr::Rat(r)) => Ok(r.denom as f64),
            _ => Ok(1.0),
        },
        // Symbolic ops: return error (should have been simplified already)
        BuiltinFn::Diff
        | BuiltinFn::Integrate
        | BuiltinFn::Taylor
        | BuiltinFn::Solve
        | BuiltinFn::Limit
        | BuiltinFn::Expand
        | BuiltinFn::Simplify => Err(CalcError::InvalidArgument(format!(
            "{} could not be evaluated numerically",
            f.name()
        ))),
        BuiltinFn::Det
        | BuiltinFn::Tr
        | BuiltinFn::Transpose
        | BuiltinFn::Inv
        | BuiltinFn::Zeros
        | BuiltinFn::Ones
        | BuiltinFn::Eye
        | BuiltinFn::Dot
        | BuiltinFn::Cross
        | BuiltinFn::Range => Err(CalcError::InvalidArgument(format!(
            "{} requires matrix/list context",
            f.name()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::simplify::simplify;

    fn ev(src: &str) -> f64 {
        let ctx = Context::new();
        eval(&simplify(parse(src).unwrap()), &ctx).unwrap()
    }

    fn ev_ctx(src: &str, vars: &[(&str, f64)]) -> f64 {
        let mut ctx = Context::new();
        for (k, v) in vars {
            ctx.set(k, *v);
        }
        eval(&parse(src).unwrap(), &ctx).unwrap()
    }

    #[test]
    fn basic_arithmetic() {
        assert!((ev("1 + 2") - 3.0).abs() < 1e-12);
        assert!((ev("10 - 3") - 7.0).abs() < 1e-12);
        assert!((ev("4 * 5") - 20.0).abs() < 1e-12);
        assert!((ev("10 / 4") - 2.5).abs() < 1e-12);
    }

    #[test]
    fn constants() {
        assert!((ev("pi") - core::f64::consts::PI).abs() < 1e-12);
        assert!((ev("e") - core::f64::consts::E).abs() < 1e-12);
    }

    #[test]
    fn trig() {
        assert!(ev("sin(0)").abs() < 1e-12);
        assert!((ev("cos(0)") - 1.0).abs() < 1e-12);
    }

    #[test]
    fn power() {
        assert!((ev("2^10") - 1024.0).abs() < 1e-12);
        assert!((ev("4^0.5") - 2.0).abs() < 1e-10);
    }

    #[test]
    fn variable_substitution() {
        assert!((ev_ctx("x^2 + 1", &[("x", 3.0)]) - 10.0).abs() < 1e-12);
    }

    #[test]
    fn factorial_eval() {
        assert!((ev("5!") - 120.0).abs() < 1e-12);
    }

    #[test]
    fn modulo() {
        assert!((ev("10 % 3") - 1.0).abs() < 1e-12);
    }

    #[test]
    fn user_fn_eval() {
        let mut env = Env::new();
        env.set_fn(
            "square",
            crate::env::UserFn {
                params: alloc::vec!["x".into()],
                body: parse("x^2").unwrap(),
            },
        );
        let expr = parse("square(3)").unwrap();
        let result = eval_env(&expr, &env).unwrap();
        assert!((result - 9.0).abs() < 1e-12);
    }
}
