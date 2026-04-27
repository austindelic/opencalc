use crate::expr::{BuiltinFn, Expr};
use crate::subst::{contains_var, subst};
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

/// Symbolically differentiate `expr` with respect to `var`.
pub fn diff(expr: &Expr, var: &str) -> Expr {
    match expr {
        Expr::Rat(_) | Expr::Float(_) | Expr::Const(_) => Expr::zero(),
        Expr::Var(name) => {
            if name == var {
                Expr::one()
            } else {
                Expr::zero()
            }
        }
        Expr::Neg(inner) => Expr::Neg(Box::new(diff(inner, var))),
        Expr::Add(terms) => Expr::Add(terms.iter().map(|t| diff(t, var)).collect()),
        Expr::Mul(factors) => {
            // d/dx(f·g·h…) = f'·g·h + f·g'·h + …
            let mut sum_terms = Vec::new();
            for i in 0..factors.len() {
                let di = diff(&factors[i], var);
                let mut term = factors.clone();
                term[i] = di;
                sum_terms.push(Expr::Mul(term));
            }
            Expr::Add(sum_terms)
        }
        Expr::Pow(base, exp) => {
            let in_base = contains_var(base, var);
            let in_exp = contains_var(exp, var);
            match (in_base, in_exp) {
                (false, false) => Expr::zero(),
                (true, false) => {
                    // d/dx f^n = n·f^(n-1)·f'
                    let exp_m1 = Expr::Add(vec![*exp.clone(), Expr::neg_one()]);
                    Expr::Mul(vec![
                        *exp.clone(),
                        Expr::Pow(base.clone(), Box::new(exp_m1)),
                        diff(base, var),
                    ])
                }
                (false, true) => {
                    // d/dx a^g = a^g · ln(a) · g'
                    Expr::Mul(vec![
                        Expr::Pow(base.clone(), exp.clone()),
                        Expr::Call(BuiltinFn::Ln, vec![*base.clone()]),
                        diff(exp, var),
                    ])
                }
                (true, true) => {
                    // d/dx f^g = f^g·(g'·ln(f) + g·f'/f)
                    let t1 = Expr::Mul(vec![
                        Expr::Pow(base.clone(), exp.clone()),
                        diff(exp, var),
                        Expr::Call(BuiltinFn::Ln, vec![*base.clone()]),
                    ]);
                    let t2 = Expr::Mul(vec![
                        *exp.clone(),
                        Expr::Pow(
                            base.clone(),
                            Box::new(Expr::Add(vec![*exp.clone(), Expr::neg_one()])),
                        ),
                        diff(base, var),
                    ]);
                    Expr::Add(vec![t1, t2])
                }
            }
        }
        Expr::Call(func, args) => diff_builtin(func, args, var),
        // User fn call — return unevaluated diff node
        e => Expr::Call(BuiltinFn::Diff, vec![e.clone(), Expr::Var(var.into())]),
    }
}

/// Chain-rule helper: f'(u) · u'
fn chain(f_prime: Expr, u: &Expr, var: &str) -> Expr {
    let du = diff(u, var);
    if du.is_zero() {
        return Expr::zero();
    }
    if du.is_one() {
        return f_prime;
    }
    Expr::Mul(vec![f_prime, du])
}

fn diff_builtin(func: &BuiltinFn, args: &[Expr], var: &str) -> Expr {
    let u = args.first().cloned().unwrap_or_else(Expr::zero);
    let v = args.get(1).cloned().unwrap_or_else(Expr::zero);

    match func {
        BuiltinFn::Sin => chain(Expr::Call(BuiltinFn::Cos, vec![u.clone()]), &u, var),
        BuiltinFn::Cos => chain(
            Expr::Neg(Box::new(Expr::Call(BuiltinFn::Sin, vec![u.clone()]))),
            &u,
            var,
        ),
        BuiltinFn::Tan => chain(
            Expr::Pow(
                Box::new(Expr::Call(BuiltinFn::Cos, vec![u.clone()])),
                Box::new(Expr::integer(-2)),
            ),
            &u,
            var,
        ),
        BuiltinFn::Asin => chain(
            Expr::Pow(
                Box::new(Expr::Add(vec![
                    Expr::one(),
                    Expr::Neg(Box::new(Expr::Pow(
                        Box::new(u.clone()),
                        Box::new(Expr::integer(2)),
                    ))),
                ])),
                Box::new(Expr::rational(-1, 2)),
            ),
            &u,
            var,
        ),
        BuiltinFn::Acos => chain(
            Expr::Neg(Box::new(Expr::Pow(
                Box::new(Expr::Add(vec![
                    Expr::one(),
                    Expr::Neg(Box::new(Expr::Pow(
                        Box::new(u.clone()),
                        Box::new(Expr::integer(2)),
                    ))),
                ])),
                Box::new(Expr::rational(-1, 2)),
            ))),
            &u,
            var,
        ),
        BuiltinFn::Atan => chain(
            Expr::Pow(
                Box::new(Expr::Add(vec![
                    Expr::one(),
                    Expr::Pow(Box::new(u.clone()), Box::new(Expr::integer(2))),
                ])),
                Box::new(Expr::neg_one()),
            ),
            &u,
            var,
        ),
        BuiltinFn::Sinh => chain(Expr::Call(BuiltinFn::Cosh, vec![u.clone()]), &u, var),
        BuiltinFn::Cosh => chain(Expr::Call(BuiltinFn::Sinh, vec![u.clone()]), &u, var),
        BuiltinFn::Tanh => chain(
            Expr::Add(vec![
                Expr::one(),
                Expr::Neg(Box::new(Expr::Pow(
                    Box::new(Expr::Call(BuiltinFn::Tanh, vec![u.clone()])),
                    Box::new(Expr::integer(2)),
                ))),
            ]),
            &u,
            var,
        ),
        BuiltinFn::Asinh => chain(
            Expr::Pow(
                Box::new(Expr::Add(vec![
                    Expr::Pow(Box::new(u.clone()), Box::new(Expr::integer(2))),
                    Expr::one(),
                ])),
                Box::new(Expr::rational(-1, 2)),
            ),
            &u,
            var,
        ),
        BuiltinFn::Acosh => chain(
            Expr::Pow(
                Box::new(Expr::Add(vec![
                    Expr::Pow(Box::new(u.clone()), Box::new(Expr::integer(2))),
                    Expr::neg_one(),
                ])),
                Box::new(Expr::rational(-1, 2)),
            ),
            &u,
            var,
        ),
        BuiltinFn::Atanh => chain(
            Expr::Pow(
                Box::new(Expr::Add(vec![
                    Expr::one(),
                    Expr::Neg(Box::new(Expr::Pow(
                        Box::new(u.clone()),
                        Box::new(Expr::integer(2)),
                    ))),
                ])),
                Box::new(Expr::neg_one()),
            ),
            &u,
            var,
        ),
        BuiltinFn::Exp => chain(Expr::Call(BuiltinFn::Exp, vec![u.clone()]), &u, var),
        BuiltinFn::Ln => chain(
            Expr::Pow(Box::new(u.clone()), Box::new(Expr::neg_one())),
            &u,
            var,
        ),
        BuiltinFn::Log if args.len() == 2 => {
            // log(base, x): d/dx = x' / (x · ln(base))
            chain(
                Expr::Pow(
                    Box::new(Expr::Mul(vec![
                        v.clone(),
                        Expr::Call(BuiltinFn::Ln, vec![u.clone()]),
                    ])),
                    Box::new(Expr::neg_one()),
                ),
                &v,
                var,
            )
        }
        BuiltinFn::Log => chain(
            Expr::Pow(Box::new(u.clone()), Box::new(Expr::neg_one())),
            &u,
            var,
        ),
        BuiltinFn::Log10 => chain(
            Expr::Pow(
                Box::new(Expr::Mul(vec![
                    u.clone(),
                    Expr::Call(BuiltinFn::Ln, vec![Expr::integer(10)]),
                ])),
                Box::new(Expr::neg_one()),
            ),
            &u,
            var,
        ),
        BuiltinFn::Log2 => chain(
            Expr::Pow(
                Box::new(Expr::Mul(vec![
                    u.clone(),
                    Expr::Call(BuiltinFn::Ln, vec![Expr::integer(2)]),
                ])),
                Box::new(Expr::neg_one()),
            ),
            &u,
            var,
        ),
        BuiltinFn::Sqrt => chain(
            Expr::Mul(vec![
                Expr::rational(1, 2),
                Expr::Pow(Box::new(u.clone()), Box::new(Expr::rational(-1, 2))),
            ]),
            &u,
            var,
        ),
        BuiltinFn::Cbrt => chain(
            Expr::Mul(vec![
                Expr::rational(1, 3),
                Expr::Pow(Box::new(u.clone()), Box::new(Expr::rational(-2, 3))),
            ]),
            &u,
            var,
        ),
        BuiltinFn::Abs => chain(Expr::Call(BuiltinFn::Sign, vec![u.clone()]), &u, var),

        // ── Fundamental Theorem of Calculus ───────────────────────────────────
        // integral(f(t), t, a, g(x))  →  f(g(x)) · g'(x)
        // integral(f(t), t, g(x), b)  →  -f(g(x)) · g'(x)
        BuiltinFn::Integral if args.len() == 4 => {
            let (integrand, t_expr, lower, upper) = (&args[0], &args[1], &args[2], &args[3]);
            if let Expr::Var(t) = t_expr {
                let upper_dep = contains_var(upper, var);
                let lower_dep = contains_var(lower, var);
                match (lower_dep, upper_dep) {
                    (false, true) => {
                        // d/dx ∫(a→g(x)) f(t) dt = f(g(x)) · g'(x)
                        let f_at_upper = subst(integrand, t, upper);
                        chain(f_at_upper, upper, var)
                    }
                    (true, false) => {
                        // d/dx ∫(g(x)→b) f(t) dt = -f(g(x)) · g'(x)
                        let f_at_lower = subst(integrand, t, lower);
                        Expr::Neg(Box::new(chain(f_at_lower, lower, var)))
                    }
                    (true, true) => {
                        // Split: ∫(g→h) = ∫(0→h) - ∫(0→g), differentiate each
                        let f_upper = subst(integrand, t, upper);
                        let f_lower = subst(integrand, t, lower);
                        Expr::Add(vec![
                            chain(f_upper, upper, var),
                            Expr::Neg(Box::new(chain(f_lower, lower, var))),
                        ])
                    }
                    (false, false) => Expr::zero(),
                }
            } else {
                Expr::Call(
                    BuiltinFn::Diff,
                    vec![
                        Expr::Call(BuiltinFn::Integral, args.to_vec()),
                        Expr::Var(var.into()),
                    ],
                )
            }
        }

        _ => Expr::Call(
            BuiltinFn::Diff,
            vec![
                Expr::Call(func.clone(), args.to_vec()),
                Expr::Var(var.into()),
            ],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::{eval, Context};
    use crate::parser::parse;
    use crate::simplify::simplify;

    #[test]
    fn constant_derivative_is_zero() {
        let e = parse("5").unwrap();
        assert!(diff(&e, "x").is_zero());
    }

    #[test]
    fn var_derivative_is_one() {
        let e = parse("x").unwrap();
        let d = simplify(diff(&e, "x"));
        let ctx = Context::new();
        let v = eval(&d, &ctx).unwrap_or(f64::NAN);
        assert!((v - 1.0).abs() < 1e-12, "d/dx(x) should be 1, got {}", v);
    }

    #[test]
    fn polynomial_derivative() {
        // d/dx(x^2) = 2x; at x=3 → 6
        let e = parse("x^2").unwrap();
        let d = simplify(diff(&e, "x"));
        let mut ctx = Context::new();
        ctx.set("x", 3.0);
        let v = eval(&d, &ctx).unwrap();
        assert!(
            (v - 6.0).abs() < 1e-10,
            "d/dx(x^2) at x=3 should be 6, got {}",
            v
        );
    }

    #[test]
    fn sin_derivative() {
        // d/dx(sin(x)) = cos(x); at x=0 → 1
        let e = parse("sin(x)").unwrap();
        let d = simplify(diff(&e, "x"));
        let mut ctx = Context::new();
        ctx.set("x", 0.0);
        let v = eval(&d, &ctx).unwrap();
        assert!((v - 1.0).abs() < 1e-12);
    }
}
