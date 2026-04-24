use alloc::vec;
use alloc::vec::Vec;
use crate::error::CalcError;
use crate::expr::Expr;
use crate::subst::subst;
use crate::eval::{eval, Context};


/// Try to solve `expr = 0` symbolically or numerically for `var`.
/// Returns a `List` of solutions.
pub fn solve(expr: &Expr, var: &str) -> Result<Expr, CalcError> {
    // Try analytic first
    if let Some(roots) = solve_analytic(expr, var) {
        return Ok(Expr::List(roots));
    }
    // Fall back to Newton-Raphson
    let roots = newton_raphson(expr, var)?;
    Ok(Expr::List(roots))
}

// ── Analytic (polynomial) ─────────────────────────────────────────────────────

fn collect_poly(expr: &Expr, var: &str) -> Option<Vec<f64>> {
    // Returns coefficients [a0, a1, a2, ...] of a polynomial in var
    // We evaluate the expression at multiple points and use Vandermonde
    // For simplicity: collect terms of the form c*var^n
    let ctx = Context::new();
    match polynomial_degree(expr, var) {
        None => None,
        Some(deg) if deg > 4 => None,
        Some(deg) => {
            let mut coeffs = vec![0.0f64; deg + 1];
            // Sample at deg+1 different points to extract coefficients
            let points: Vec<f64> = (0..=deg).map(|i| i as f64).collect();
            let values: Vec<f64> = points.iter().map(|&x| {
                let xe = Expr::Float(x);
                eval(&subst(expr, var, &xe), &ctx).unwrap_or(f64::NAN)
            }).collect();
            if values.iter().any(|v| v.is_nan()) { return None; }

            // Solve for coefficients via forward differences
            // Use Lagrange interpolation approach
            vandermonde_solve(&points, &values, &mut coeffs)?;
            Some(coeffs)
        }
    }
}

fn polynomial_degree(expr: &Expr, var: &str) -> Option<usize> {
    match expr {
        Expr::Var(v) if v == var => Some(1),
        Expr::Rat(_) | Expr::Float(_) | Expr::Const(_) => Some(0),
        Expr::Var(_) => Some(0),
        Expr::Neg(inner) => polynomial_degree(inner, var),
        Expr::Add(terms) => terms.iter()
            .map(|t| polynomial_degree(t, var))
            .try_fold(0, |acc, d| d.map(|d| acc.max(d))),
        Expr::Mul(factors) => factors.iter()
            .map(|f| polynomial_degree(f, var))
            .try_fold(0, |acc, d| d.map(|d| acc + d)),
        Expr::Pow(base, exp) => {
            if matches!(base.as_ref(), Expr::Var(v) if v == var) {
                if let Expr::Rat(r) = exp.as_ref() {
                    if r.denom == 1 && r.numer >= 0 { return Some(r.numer as usize); }
                }
            }
            if polynomial_degree(base, var)? == 0 { Some(0) } else { None }
        }
        _ => None,
    }
}

fn vandermonde_solve(xs: &[f64], ys: &[f64], coeffs: &mut [f64]) -> Option<()> {
    let n = xs.len();
    // Build Vandermonde matrix and solve via Gaussian elimination
    let mut mat: Vec<Vec<f64>> = xs.iter().map(|&x| {
        let row: Vec<f64> = (0..n).map(|j| libm::pow(x, j as f64)).collect();
        row
    }).collect();
    let mut rhs: Vec<f64> = ys.to_vec();

    for col in 0..n {
        // Find pivot
        let pivot = (col..n).max_by(|&a, &b| {
            mat[a][col].abs().partial_cmp(&mat[b][col].abs())
                .unwrap_or(core::cmp::Ordering::Equal)
        })?;
        mat.swap(col, pivot);
        rhs.swap(col, pivot);
        let pv = mat[col][col];
        if pv.abs() < 1e-12 { return None; }
        for j in col..n { mat[col][j] /= pv; }
        rhs[col] /= pv;
        for row in 0..n {
            if row == col { continue; }
            let factor = mat[row][col];
            for j in col..n { mat[row][j] -= factor * mat[col][j]; }
            rhs[row] -= factor * rhs[col];
        }
    }
    for (i, c) in coeffs.iter_mut().enumerate() { *c = rhs[i]; }
    Some(())
}

fn solve_analytic(expr: &Expr, var: &str) -> Option<Vec<Expr>> {
    let coeffs = collect_poly(expr, var)?;
    match coeffs.len() {
        1 => {
            // c0 = 0  → any x (or no solution if c0 ≠ 0)
            None
        }
        2 => {
            // c0 + c1*x = 0  →  x = -c0/c1
            let c0 = coeffs[0];
            let c1 = coeffs[1];
            if c1.abs() < 1e-12 { return None; }
            Some(vec![Expr::Float(-c0 / c1)])
        }
        3 => {
            // c0 + c1*x + c2*x^2 = 0
            let (a, b, c) = (coeffs[2], coeffs[1], coeffs[0]);
            if a.abs() < 1e-12 { return solve_analytic_linear(b, c); }
            let disc = b * b - 4.0 * a * c;
            if disc < 0.0 { return Some(vec![]); }
            let sq = libm::sqrt(disc);
            Some(vec![
                Expr::Float((-b + sq) / (2.0 * a)),
                Expr::Float((-b - sq) / (2.0 * a)),
            ])
        }
        _ => None,
    }
}

fn solve_analytic_linear(b: f64, c: f64) -> Option<Vec<Expr>> {
    if b.abs() < 1e-12 { return None; }
    Some(vec![Expr::Float(-c / b)])
}

// ── Numerical (Newton-Raphson) ────────────────────────────────────────────────

fn newton_raphson(expr: &Expr, var: &str) -> Result<Vec<Expr>, CalcError> {
    use crate::diff::diff;
    use crate::simplify::simplify;
    let ctx = Context::new();
    let deriv = simplify(diff(expr, var));
    let mut roots = Vec::new();

    // Try multiple starting points to find distinct roots
    let starts: &[f64] = &[-10.0, -3.0, -1.0, 0.0, 1.0, 3.0, 10.0];
    'outer: for &x0 in starts {
        let mut x = x0;
        for _ in 0..50 {
            let xe = Expr::Float(x);
            let fx  = eval(&subst(expr, var, &xe), &ctx).unwrap_or(f64::NAN);
            let fpx = eval(&subst(&deriv, var, &xe), &ctx).unwrap_or(f64::NAN);
            if fpx.abs() < 1e-15 { break; }
            let x_new = x - fx / fpx;
            if (x_new - x).abs() < 1e-10 {
                x = x_new;
                let fx_final = eval(&subst(expr, var, &Expr::Float(x)), &ctx).unwrap_or(f64::NAN);
                if fx_final.abs() < 1e-8 {
                    // Check if this root is already found
                    if roots.iter().all(|r: &Expr| match r {
                        Expr::Float(v) => (v - x).abs() > 1e-6,
                        _ => true,
                    }) {
                        roots.push(Expr::Float(x));
                        if roots.len() >= 6 { break 'outer; }
                    }
                }
                break;
            }
            x = x_new;
        }
    }
    Ok(roots)
}
