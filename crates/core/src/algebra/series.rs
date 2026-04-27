use crate::diff::diff;
use crate::error::CalcError;
use crate::expr::Expr;
use crate::subst::subst;
use alloc::vec;
use alloc::vec::Vec;

/// Compute Taylor series of `expr` around `point` to `order` terms.
/// Returns a polynomial in `var`.
pub fn taylor(expr: &Expr, var: &str, point: &Expr, order: usize) -> Result<Expr, CalcError> {
    let mut terms = Vec::new();
    let mut deriv = expr.clone();
    let mut factorial: u64 = 1;

    for n in 0..=order {
        if n > 0 {
            factorial *= n as u64;
        }

        // Evaluate deriv at point
        let val_at_point = subst(&deriv, var, point);

        // term = val_at_point / n! * (x - point)^n
        let coeff = if factorial == 1 {
            val_at_point
        } else {
            Expr::Mul(vec![Expr::rational(1, factorial as i64), val_at_point])
        };

        let term = if n == 0 {
            coeff
        } else if n == 1 {
            Expr::Mul(vec![
                coeff,
                Expr::Add(vec![
                    Expr::Var(var.into()),
                    Expr::Neg(alloc::boxed::Box::new(point.clone())),
                ]),
            ])
        } else {
            Expr::Mul(vec![
                coeff,
                Expr::Pow(
                    alloc::boxed::Box::new(Expr::Add(vec![
                        Expr::Var(var.into()),
                        Expr::Neg(alloc::boxed::Box::new(point.clone())),
                    ])),
                    alloc::boxed::Box::new(Expr::integer(n as i64)),
                ),
            ])
        };

        terms.push(term);
        deriv = diff(&deriv, var);
    }

    Ok(Expr::Add(terms))
}
