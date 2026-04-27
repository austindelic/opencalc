use crate::error::CalcError;
use crate::expr::{BuiltinFn, Expr};
use crate::subst::contains_var;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

/// Attempt symbolic indefinite integration of `expr` w.r.t. `var`.
/// Returns `None` if no pattern matches.
pub fn integrate(expr: &Expr, var: &str) -> Option<Expr> {
    match expr {
        // ∫c dx = c·x  (constant)
        e if !contains_var(e, var) => Some(Expr::Mul(vec![e.clone(), Expr::Var(var.into())])),

        // ∫x dx = x²/2
        Expr::Var(v) if v == var => Some(Expr::Mul(vec![
            Expr::rational(1, 2),
            Expr::Pow(Box::new(Expr::Var(var.into())), Box::new(Expr::integer(2))),
        ])),

        // ∫x^n dx = x^(n+1)/(n+1)   n ≠ -1
        Expr::Pow(base, exp) if matches!(base.as_ref(), Expr::Var(v) if v == var) => {
            if exp.as_ref() == &Expr::neg_one() {
                // ∫1/x dx = ln|x|
                return Some(Expr::Call(
                    BuiltinFn::Ln,
                    vec![Expr::Call(BuiltinFn::Abs, vec![Expr::Var(var.into())])],
                ));
            }
            match exp.as_ref() {
                Expr::Rat(r) => {
                    let n1 = crate::rational::Rational::new(r.numer + r.denom, r.denom);
                    Some(Expr::Mul(vec![
                        Expr::Rat(crate::rational::Rational::new(n1.denom, n1.numer)),
                        Expr::Pow(Box::new(Expr::Var(var.into())), Box::new(Expr::Rat(n1))),
                    ]))
                }
                _ => None,
            }
        }

        // ∫c·f dx = c·∫f dx   (scalar factor)
        Expr::Mul(factors) => {
            let (consts, vars): (Vec<_>, Vec<_>) =
                factors.iter().partition(|f| !contains_var(f, var));
            if consts.is_empty() {
                return None;
            }
            let inner = if vars.len() == 1 {
                vars[0].clone()
            } else {
                Expr::Mul(vars.into_iter().cloned().collect())
            };
            let antideriv = integrate(&inner, var)?;
            let coeff = if consts.len() == 1 {
                consts[0].clone()
            } else {
                Expr::Mul(consts.into_iter().cloned().collect())
            };
            Some(Expr::Mul(vec![coeff.clone(), antideriv]))
        }

        // ∫(f + g) dx = ∫f dx + ∫g dx
        Expr::Add(terms) => {
            let parts: Vec<Option<Expr>> = terms.iter().map(|t| integrate(t, var)).collect();
            if parts.iter().any(|p| p.is_none()) {
                return None;
            }
            Some(Expr::Add(parts.into_iter().map(|p| p.unwrap()).collect()))
        }

        // ∫sin(x) dx = -cos(x)
        Expr::Call(BuiltinFn::Sin, args) if args.len() == 1 && args[0] == Expr::Var(var.into()) => {
            Some(Expr::Neg(Box::new(Expr::Call(
                BuiltinFn::Cos,
                vec![Expr::Var(var.into())],
            ))))
        }
        // ∫cos(x) dx = sin(x)
        Expr::Call(BuiltinFn::Cos, args) if args.len() == 1 && args[0] == Expr::Var(var.into()) => {
            Some(Expr::Call(BuiltinFn::Sin, vec![Expr::Var(var.into())]))
        }
        // ∫exp(x) dx = exp(x)
        Expr::Call(BuiltinFn::Exp, args) if args.len() == 1 && args[0] == Expr::Var(var.into()) => {
            Some(Expr::Call(BuiltinFn::Exp, vec![Expr::Var(var.into())]))
        }
        // ∫ln(x) dx = x·ln(x) - x
        Expr::Call(BuiltinFn::Ln, args) if args.len() == 1 && args[0] == Expr::Var(var.into()) => {
            let x = Expr::Var(var.into());
            Some(Expr::Add(vec![
                Expr::Mul(vec![x.clone(), Expr::Call(BuiltinFn::Ln, vec![x.clone()])]),
                Expr::Neg(Box::new(x)),
            ]))
        }
        // ∫sqrt(x) dx = (2/3)·x^(3/2)
        Expr::Call(BuiltinFn::Sqrt, args)
            if args.len() == 1 && args[0] == Expr::Var(var.into()) =>
        {
            Some(Expr::Mul(vec![
                Expr::rational(2, 3),
                Expr::Pow(
                    Box::new(Expr::Var(var.into())),
                    Box::new(Expr::rational(3, 2)),
                ),
            ]))
        }
        _ => None,
    }
}

/// Numerical integration via Simpson's rule (n must be even).
pub fn nintegrate(expr: &Expr, var: &str, a: f64, b: f64, n: usize) -> Result<f64, CalcError> {
    use crate::eval::{eval, Context};
    use crate::subst::subst;
    let n = if n % 2 == 0 { n } else { n + 1 };
    let h = (b - a) / n as f64;
    let mut sum = 0.0f64;
    for i in 0..=n {
        let x = a + i as f64 * h;
        let xe = Expr::Float(x);
        let fx = eval(&subst(expr, var, &xe), &Context::new())?;
        let w = if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        sum += w * fx;
    }
    Ok(sum * h / 3.0)
}
