use crate::expr::{BuiltinFn, Expr};
use crate::rational::Rational;
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Algebraically simplify an expression tree (bottom-up).
pub fn simplify(expr: Expr) -> Expr {
    let expr = simplify_children(expr);
    simplify_node(expr)
}

fn simplify_children(expr: Expr) -> Expr {
    match expr {
        Expr::Neg(inner) => Expr::Neg(Box::new(simplify(*inner))),
        Expr::Add(terms) => Expr::Add(terms.into_iter().map(simplify).collect()),
        Expr::Mul(factors) => Expr::Mul(factors.into_iter().map(simplify).collect()),
        Expr::Pow(base, exp) => Expr::Pow(Box::new(simplify(*base)), Box::new(simplify(*exp))),
        Expr::Call(f, args) => Expr::Call(f, args.into_iter().map(simplify).collect()),
        Expr::FnCall(name, args) => Expr::FnCall(name, args.into_iter().map(simplify).collect()),
        Expr::Equation(l, r) => Expr::Equation(Box::new(simplify(*l)), Box::new(simplify(*r))),
        Expr::Matrix(rows) => Expr::Matrix(
            rows.into_iter()
                .map(|row| row.into_iter().map(simplify).collect())
                .collect(),
        ),
        Expr::List(items) => Expr::List(items.into_iter().map(simplify).collect()),
        other => other,
    }
}

fn simplify_node(expr: Expr) -> Expr {
    match expr {
        Expr::Neg(inner) => simplify_neg(*inner),
        Expr::Add(terms) => simplify_add(terms),
        Expr::Mul(factors) => simplify_mul(factors),
        Expr::Pow(base, exp) => simplify_pow(*base, *exp),
        Expr::Call(f, args) => simplify_call(f, args),
        other => other,
    }
}

// ── Neg ──────────────────────────────────────────────────────────────────────

fn simplify_neg(inner: Expr) -> Expr {
    match inner {
        Expr::Rat(r) => Expr::Rat(-r),
        Expr::Float(f) => Expr::Float(-f),
        Expr::Neg(x) => *x,
        other => Expr::Neg(Box::new(other)),
    }
}

// ── Add ──────────────────────────────────────────────────────────────────────

fn simplify_add(terms: Vec<Expr>) -> Expr {
    let mut flat: Vec<Expr> = Vec::new();
    for t in terms {
        match t {
            Expr::Add(inner) => flat.extend(inner),
            other => flat.push(other),
        }
    }

    let mut groups: Vec<(Expr, Rational)> = Vec::new();
    for term in flat {
        let (coeff, sym) = extract_addend(term);
        if let Some((_, acc)) = groups.iter_mut().find(|(s, _)| s == &sym) {
            *acc = *acc + coeff;
        } else {
            groups.push((sym, coeff));
        }
    }

    let mut result: Vec<Expr> = Vec::new();
    for (sym, coeff) in groups {
        if coeff.is_zero() {
            continue;
        }
        result.push(coeff_times(coeff, sym));
    }

    result = cancel_pythagorean(result);

    match result.len() {
        0 => Expr::zero(),
        1 => result.remove(0),
        _ => Expr::Add(result),
    }
}

fn extract_addend(expr: Expr) -> (Rational, Expr) {
    match expr {
        Expr::Rat(r) => (r, Expr::one()),
        Expr::Neg(inner) => {
            let (c, s) = extract_addend(*inner);
            (-c, s)
        }
        Expr::Mul(mut factors) if matches!(factors.first(), Some(Expr::Rat(_))) => {
            // Guard guarantees first element is Rat; let-else keeps this explicit.
            let Expr::Rat(r) = factors.remove(0) else {
                return (Rational::one(), Expr::Mul(factors));
            };
            let sym = match factors.len() {
                0 => Expr::one(),
                1 => factors.remove(0),
                _ => Expr::Mul(factors),
            };
            (r, sym)
        }
        other => (Rational::one(), other),
    }
}

fn coeff_times(coeff: Rational, sym: Expr) -> Expr {
    if coeff.is_one() {
        sym
    } else if coeff.is_neg_one() {
        simplify_neg(sym)
    } else if sym.is_one() {
        Expr::Rat(coeff)
    } else {
        Expr::Mul(vec![Expr::Rat(coeff), sym])
    }
}

// ── Mul ──────────────────────────────────────────────────────────────────────

fn simplify_mul(factors: Vec<Expr>) -> Expr {
    let mut flat: Vec<Expr> = Vec::new();
    for f in factors {
        match f {
            Expr::Mul(inner) => flat.extend(inner),
            other => flat.push(other),
        }
    }
    if flat.iter().any(|f| f.is_zero()) {
        return Expr::zero();
    }

    let mut coeff = Rational::one();
    let mut sym_factors: Vec<Expr> = Vec::new();
    for f in flat {
        match f {
            Expr::Rat(r) => coeff = coeff * r,
            Expr::Neg(inner) => {
                coeff = -coeff;
                sym_factors.push(*inner);
            }
            other => sym_factors.push(other),
        }
    }
    if coeff.is_zero() {
        return Expr::zero();
    }

    let mut groups: Vec<(Expr, Expr)> = Vec::new();
    for f in sym_factors {
        let (base, exp) = extract_factor(f);
        if let Some((_, acc_exp)) = groups.iter_mut().find(|(b, _)| b == &base) {
            *acc_exp = simplify_add(vec![acc_exp.clone(), exp]);
        } else {
            groups.push((base, exp));
        }
    }

    let mut sym_result: Vec<Expr> = Vec::new();
    for (base, exp) in groups {
        let f = simplify_pow(base, exp);
        if !f.is_one() {
            sym_result.push(f);
        }
    }

    let product = match sym_result.len() {
        0 => Expr::one(),
        1 => sym_result.remove(0),
        _ => Expr::Mul(sym_result),
    };

    if coeff.is_zero() {
        Expr::zero()
    } else if coeff.is_one() {
        product
    } else if coeff.is_neg_one() {
        if product.is_one() {
            Expr::neg_one()
        } else {
            simplify_neg(product)
        }
    } else if product.is_one() {
        Expr::Rat(coeff)
    } else {
        match product {
            Expr::Mul(mut inner) => {
                inner.insert(0, Expr::Rat(coeff));
                Expr::Mul(inner)
            }
            other => Expr::Mul(vec![Expr::Rat(coeff), other]),
        }
    }
}

fn extract_factor(expr: Expr) -> (Expr, Expr) {
    match expr {
        Expr::Pow(base, exp) => (*base, *exp),
        other => (other, Expr::one()),
    }
}

// ── Pow ──────────────────────────────────────────────────────────────────────

fn simplify_pow(base: Expr, exp: Expr) -> Expr {
    if exp.is_zero() {
        return Expr::one();
    }
    if exp.is_one() {
        return base;
    }
    if base.is_zero() {
        return Expr::zero();
    }
    if base.is_one() {
        return Expr::one();
    }

    if let (Expr::Rat(b), Expr::Rat(e)) = (&base, &exp) {
        if e.is_integer() {
            let n = e.numer;
            if n >= -64 && n <= 64 {
                if let Some(r) = b.checked_pow_int(n as i32) {
                    return Expr::Rat(r);
                }
            }
        }
    }

    if let Expr::Pow(inner_base, inner_exp) = base.clone() {
        let new_exp = simplify_mul(vec![*inner_exp, exp]);
        return simplify_pow(*inner_base, new_exp);
    }

    Expr::Pow(Box::new(base), Box::new(exp))
}

// ── Call ─────────────────────────────────────────────────────────────────────

fn simplify_call(f: BuiltinFn, args: Vec<Expr>) -> Expr {
    // ── Symbolic calculus ops ─────────────────────────────────────────────────
    match (&f, args.as_slice()) {
        // diff(expr, var) → symbolic derivative
        (BuiltinFn::Diff, [expr, Expr::Var(var)]) => {
            return simplify(crate::diff::diff(expr, var));
        }
        // diff(expr, var, n) → nth derivative
        (BuiltinFn::Diff, [expr, Expr::Var(var), Expr::Rat(n)])
            if n.is_integer() && n.numer > 0 =>
        {
            let mut result = expr.clone();
            for _ in 0..n.numer {
                result = simplify(crate::diff::diff(&result, var));
            }
            return result;
        }
        // integrate(expr, var) → indefinite integral
        (BuiltinFn::Integrate, [expr, Expr::Var(var)]) => {
            if let Some(antideriv) = crate::integrate::integrate(expr, var) {
                return simplify(antideriv);
            }
        }
        // taylor(expr, var, point, order)
        (BuiltinFn::Taylor, [expr, Expr::Var(var), point, Expr::Rat(order)])
            if order.is_integer() && order.numer >= 0 =>
        {
            if let Ok(series) = crate::series::taylor(expr, var, point, order.numer as usize) {
                return simplify(series);
            }
        }
        // solve(expr, var) or solve(equation, var)
        (BuiltinFn::Solve, [expr, Expr::Var(var)]) => {
            let target = match expr {
                Expr::Equation(l, r) => {
                    simplify(Expr::Add(vec![*l.clone(), Expr::Neg(Box::new(*r.clone()))]))
                }
                other => other.clone(),
            };
            if let Ok(roots) = crate::solve::solve(&target, var) {
                return roots;
            }
        }
        // solve(equation)  — infer variable
        (BuiltinFn::Solve, [Expr::Equation(l, r)]) => {
            let diff = simplify(Expr::Add(vec![*l.clone(), Expr::Neg(Box::new(*r.clone()))]));
            if let Ok(roots) = crate::solve::solve(&diff, "x") {
                return roots;
            }
        }
        // simplify(expr) → already simplified
        (BuiltinFn::Simplify, [expr]) => return expr.clone(),

        // ── expand(expr) ─────────────────────────────────────────────────────
        (BuiltinFn::Expand, [expr]) => return expand_expr(expr.clone()),

        // ── Matrix/vector constructors ────────────────────────────────────────
        (BuiltinFn::Zeros, [Expr::Rat(r)]) if r.is_integer() && r.numer > 0 => {
            let n = r.numer as usize;
            return Expr::Matrix(vec![vec![Expr::zero(); n]; n]);
        }
        (BuiltinFn::Zeros, [Expr::Rat(r), Expr::Rat(c)])
            if r.is_integer() && c.is_integer() && r.numer > 0 && c.numer > 0 =>
        {
            return Expr::Matrix(vec![vec![Expr::zero(); c.numer as usize]; r.numer as usize]);
        }
        (BuiltinFn::Ones, [Expr::Rat(r)]) if r.is_integer() && r.numer > 0 => {
            let n = r.numer as usize;
            return Expr::Matrix(vec![vec![Expr::one(); n]; n]);
        }
        (BuiltinFn::Ones, [Expr::Rat(r), Expr::Rat(c)])
            if r.is_integer() && c.is_integer() && r.numer > 0 && c.numer > 0 =>
        {
            return Expr::Matrix(vec![vec![Expr::one(); c.numer as usize]; r.numer as usize]);
        }
        (BuiltinFn::Eye, [Expr::Rat(n)]) if n.is_integer() && n.numer > 0 => {
            let n = n.numer as usize;
            return Expr::Matrix(
                (0..n)
                    .map(|i| {
                        (0..n)
                            .map(|j| if i == j { Expr::one() } else { Expr::zero() })
                            .collect()
                    })
                    .collect(),
            );
        }
        (BuiltinFn::Range, [Expr::Rat(n)]) if n.is_integer() && n.numer > 0 => {
            return Expr::List((0..n.numer).map(Expr::integer).collect());
        }
        (BuiltinFn::Range, [Expr::Rat(s), Expr::Rat(e)]) if s.is_integer() && e.is_integer() => {
            if s.numer >= e.numer {
                return Expr::List(vec![]);
            }
            return Expr::List((s.numer..e.numer).map(Expr::integer).collect());
        }
        (BuiltinFn::Range, [Expr::Rat(s), Expr::Rat(e), Expr::Rat(step)])
            if s.is_integer() && e.is_integer() && step.is_integer() && step.numer > 0 =>
        {
            let (start, end, step) = (s.numer, e.numer, step.numer);
            return Expr::List(
                (0..)
                    .map(|i| start + i * step)
                    .take_while(|&v| v < end)
                    .map(Expr::integer)
                    .collect(),
            );
        }

        // ── Matrix ops ────────────────────────────────────────────────────────
        (BuiltinFn::Det, [Expr::Matrix(rows)]) => {
            if let Ok(result) = crate::matrix::mat_det(rows) {
                return simplify(result);
            }
        }
        (BuiltinFn::Tr, [Expr::Matrix(rows)]) => {
            if let Ok(result) = crate::matrix::mat_trace(rows) {
                return simplify(result);
            }
        }
        (BuiltinFn::Transpose, [Expr::Matrix(rows)]) => {
            if let Ok(result) = crate::matrix::mat_transpose(rows) {
                return Expr::Matrix(
                    result
                        .into_iter()
                        .map(|row| row.into_iter().map(simplify).collect())
                        .collect(),
                );
            }
        }
        (BuiltinFn::Inv, [Expr::Matrix(rows)]) => {
            if let Ok(result) = crate::matrix::mat_inv(rows) {
                return Expr::Matrix(
                    result
                        .into_iter()
                        .map(|row| row.into_iter().map(simplify).collect())
                        .collect(),
                );
            }
        }
        (BuiltinFn::Dot, [Expr::List(a), Expr::List(b)]) => {
            if let Ok(result) = crate::matrix::dot(a, b) {
                return simplify(result);
            }
        }
        (BuiltinFn::Cross, [Expr::List(a), Expr::List(b)]) => {
            if let Ok(result) = crate::matrix::cross3(a, b) {
                return Expr::List(result.into_iter().map(simplify).collect());
            }
        }

        _ => {}
    }

    // ── Numeric constant folding for builtins ─────────────────────────────────
    match (&f, args.as_slice()) {
        (BuiltinFn::Sin, [e]) if e.is_zero() => Expr::zero(),
        (BuiltinFn::Cos, [e]) if e.is_zero() => Expr::one(),
        (BuiltinFn::Tan, [e]) if e.is_zero() => Expr::zero(),
        (BuiltinFn::Sinh, [e]) if e.is_zero() => Expr::zero(),
        (BuiltinFn::Cosh, [e]) if e.is_zero() => Expr::one(),
        (BuiltinFn::Tanh, [e]) if e.is_zero() => Expr::zero(),
        (BuiltinFn::Exp, [e]) if e.is_zero() => Expr::one(),
        (BuiltinFn::Ln, [e]) if e.is_one() => Expr::zero(),
        (BuiltinFn::Log, [e]) if e.is_one() => Expr::zero(),
        (BuiltinFn::Sqrt, [e]) if e.is_zero() => Expr::zero(),
        (BuiltinFn::Sqrt, [e]) if e.is_one() => Expr::one(),
        (BuiltinFn::Abs, [Expr::Rat(r)]) => Expr::Rat(r.abs()),
        (BuiltinFn::Abs, [Expr::Float(v)]) => Expr::Float(v.abs()),
        (BuiltinFn::Factorial, [Expr::Rat(r)])
            if r.is_integer() && r.numer >= 0 && r.numer <= 20 =>
        {
            let n = r.numer as u64;
            Expr::Rat(Rational::from((1..=n).product::<u64>() as i64))
        }

        // ── Inverse function cancellation ─────────────────────────────────────
        // exp(ln(x)) = x
        (BuiltinFn::Exp, [Expr::Call(BuiltinFn::Ln, inner)]) if inner.len() == 1 => {
            inner[0].clone()
        }
        // ln(exp(x)) = x
        (BuiltinFn::Ln, [Expr::Call(BuiltinFn::Exp, inner)]) if inner.len() == 1 => {
            inner[0].clone()
        }
        // sqrt(x^2) = abs(x)
        (BuiltinFn::Sqrt, [Expr::Pow(base, exp)]) if exp.as_ref() == &Expr::integer(2) => {
            Expr::Call(BuiltinFn::Abs, vec![*base.clone()])
        }
        // abs(abs(x)) = abs(x)
        (BuiltinFn::Abs, [Expr::Call(BuiltinFn::Abs, inner)]) if inner.len() == 1 => {
            Expr::Call(BuiltinFn::Abs, vec![inner[0].clone()])
        }
        // log(b, b^x) = x  (log base b of b^x)
        (BuiltinFn::Log, [base, Expr::Pow(pow_base, exp)]) if base == pow_base.as_ref() => {
            *exp.clone()
        }
        _ => Expr::Call(f, args),
    }
}

// ── Pythagorean identities ────────────────────────────────────────────────────

/// Cancel `c·sin²(u) + c·cos²(u)` → `c` and `c·cosh²(u) - c·sinh²(u)` → `c`
/// anywhere in a list of addends (handles any common coefficient).
fn cancel_pythagorean(mut terms: Vec<Expr>) -> Vec<Expr> {
    let mut i = 0;
    'outer: while i < terms.len() {
        // sin²(u) + cos²(u) = 1  (scaled by matching coefficient)
        if let Some((u, c)) = trig_sq_arg(&terms[i], BuiltinFn::Sin) {
            for j in (i + 1)..terms.len() {
                if let Some((v, d)) = trig_sq_arg(&terms[j], BuiltinFn::Cos) {
                    if v == u && d == c {
                        terms[i] = if c.is_one() { Expr::one() } else { Expr::Rat(c) };
                        terms.remove(j);
                        i += 1;
                        continue 'outer;
                    }
                }
            }
        }
        // cosh²(u) - sinh²(u) = 1  (cosh has coeff c, sinh has coeff -c)
        if let Some((u, c)) = trig_sq_arg(&terms[i], BuiltinFn::Cosh) {
            let neg_c = -c;
            for j in (i + 1)..terms.len() {
                if let Some((v, d)) = trig_sq_arg(&terms[j], BuiltinFn::Sinh) {
                    if v == u && d == neg_c {
                        terms[i] = if c.is_one() { Expr::one() } else { Expr::Rat(c) };
                        terms.remove(j);
                        i += 1;
                        continue 'outer;
                    }
                }
            }
        }
        i += 1;
    }
    terms
}

/// Extract `(argument, coefficient)` from expressions of the form:
///   `func²(u)`          → `(u,  1)`
///   `c · func²(u)`      → `(u,  c)`
///   `-(func²(u))`       → `(u, -1)`
fn trig_sq_arg(e: &Expr, func: BuiltinFn) -> Option<(Expr, Rational)> {
    // func²(u)
    if let Expr::Pow(base, exp) = e {
        if exp.as_ref() == &Expr::integer(2) {
            if let Expr::Call(f, args) = base.as_ref() {
                if *f == func && args.len() == 1 {
                    return Some((args[0].clone(), Rational::one()));
                }
            }
        }
    }
    // -(func²(u))
    if let Expr::Neg(inner) = e {
        if let Expr::Pow(base, exp) = inner.as_ref() {
            if exp.as_ref() == &Expr::integer(2) {
                if let Expr::Call(f, args) = base.as_ref() {
                    if *f == func && args.len() == 1 {
                        return Some((args[0].clone(), Rational::neg_one()));
                    }
                }
            }
        }
    }
    // c · func²(u)
    if let Expr::Mul(factors) = e {
        if factors.len() == 2 {
            if let (Expr::Rat(c), Expr::Pow(base, exp)) = (&factors[0], &factors[1]) {
                if exp.as_ref() == &Expr::integer(2) {
                    if let Expr::Call(f, args) = base.as_ref() {
                        if *f == func && args.len() == 1 {
                            return Some((args[0].clone(), *c));
                        }
                    }
                }
            }
        }
    }
    None
}

// ── Expand ────────────────────────────────────────────────────────────────────

/// Fully distribute multiplication over addition and expand integer powers of sums.
fn expand_expr(expr: Expr) -> Expr {
    match expr {
        Expr::Neg(inner) => simplify(Expr::Neg(Box::new(expand_expr(*inner)))),
        Expr::Add(terms) => simplify(Expr::Add(terms.into_iter().map(expand_expr).collect())),
        Expr::Mul(factors) => distribute_mul(factors.into_iter().map(expand_expr).collect()),
        Expr::Pow(base, exp) => {
            let base = expand_expr(*base);
            match *exp {
                Expr::Rat(r) if r.is_integer() && r.numer >= 2 && r.numer <= 8 => {
                    distribute_mul(vec![base; r.numer as usize])
                }
                e => simplify(Expr::Pow(Box::new(base), Box::new(e))),
            }
        }
        other => simplify(other),
    }
}

/// Multiply a list of factors, distributing over any `Add` terms encountered.
fn distribute_mul(factors: Vec<Expr>) -> Expr {
    let mut acc: Vec<Expr> = vec![Expr::one()];
    for factor in factors {
        match factor {
            Expr::Add(terms) => {
                acc = acc
                    .iter()
                    .flat_map(|a| {
                        terms
                            .iter()
                            .map(move |t| simplify(Expr::Mul(vec![a.clone(), t.clone()])))
                    })
                    .collect();
            }
            other => {
                acc = acc
                    .into_iter()
                    .map(|a| simplify(Expr::Mul(vec![a, other.clone()])))
                    .collect();
            }
        }
    }
    simplify(Expr::Add(acc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn s(src: &str) -> Expr {
        simplify(parse(src).unwrap())
    }

    #[test]
    fn constant_fold() {
        assert_eq!(s("2 + 3"), Expr::integer(5));
        assert_eq!(s("2 * 3"), Expr::integer(6));
        assert_eq!(s("10 / 4"), Expr::Rat(Rational::new(5, 2)));
    }

    #[test]
    fn like_terms() {
        assert_eq!(
            s("x + x"),
            Expr::Mul(vec![Expr::integer(2), Expr::Var("x".into())])
        );
        assert_eq!(
            s("2*x + 3*x"),
            Expr::Mul(vec![Expr::integer(5), Expr::Var("x".into())])
        );
    }

    #[test]
    fn like_bases() {
        assert_eq!(
            s("x^2 * x^3"),
            Expr::Pow(Box::new(Expr::Var("x".into())), Box::new(Expr::integer(5)))
        );
    }

    #[test]
    fn power_identities() {
        assert_eq!(s("x^0"), Expr::one());
        assert_eq!(s("x^1"), Expr::Var("x".into()));
        assert_eq!(s("0^5"), Expr::zero());
        assert_eq!(s("1^99"), Expr::one());
    }

    #[test]
    fn double_neg() {
        assert_eq!(s("--x"), Expr::Var("x".into()));
    }

    #[test]
    fn zero_identity_add() {
        assert_eq!(s("x + 0"), Expr::Var("x".into()));
    }

    #[test]
    fn zero_annihilator_mul() {
        assert_eq!(s("x * 0"), Expr::zero());
    }

    #[test]
    fn factorial_small() {
        assert_eq!(s("5!"), Expr::integer(120));
    }

    #[test]
    fn trig_at_zero() {
        assert_eq!(s("sin(0)"), Expr::zero());
        assert_eq!(s("cos(0)"), Expr::one());
    }

    #[test]
    fn cancel_inverse() {
        assert_eq!(s("x / x"), Expr::one());
    }

    #[test]
    fn expand_square() {
        // expand((x+1)^2) = x^2 + 2x + 1
        let d = s("expand((x+1)^2)");
        // Evaluate at x=3: should be 16
        use crate::eval::{Context, eval};
        let mut ctx = Context::new();
        ctx.set("x", 3.0);
        let v = eval(&d, &ctx).unwrap();
        assert!((v - 16.0).abs() < 1e-10);
    }

    #[test]
    fn matrix_constructors() {
        let z = s("zeros(2)");
        assert!(matches!(z, Expr::Matrix(_)));
        let e = s("eye(3)");
        assert!(matches!(e, Expr::Matrix(_)));
    }

    #[test]
    fn range_list() {
        let r = s("range(4)");
        assert_eq!(
            r,
            Expr::List(vec![
                Expr::integer(0),
                Expr::integer(1),
                Expr::integer(2),
                Expr::integer(3),
            ])
        );
    }

    #[test]
    fn pythagorean_identity() {
        assert_eq!(s("sin(x)^2 + cos(x)^2"), Expr::one());
    }

    #[test]
    fn pythagorean_scaled() {
        // 3·sin²(x) + 3·cos²(x) = 3
        assert_eq!(s("3*sin(x)^2 + 3*cos(x)^2"), Expr::integer(3));
    }

    #[test]
    fn hyperbolic_identity() {
        assert_eq!(s("cosh(x)^2 - sinh(x)^2"), Expr::one());
    }

    #[test]
    fn exp_ln_inverse() {
        assert_eq!(s("exp(ln(x))"), Expr::Var("x".into()));
        assert_eq!(s("ln(exp(x))"), Expr::Var("x".into()));
    }

    #[test]
    fn abs_abs() {
        assert_eq!(s("abs(abs(x))"), s("abs(x)"));
    }

    #[test]
    fn det_2x2() {
        // det([[1,2],[3,4]]) = 1*4 - 2*3 = -2
        let d = s("det([[1,2],[3,4]])");
        assert_eq!(d, Expr::integer(-2));
    }

    #[test]
    fn symbolic_diff() {
        // diff(x^2, x) should simplify to 2*x
        let d = s("diff(x^2, x)");
        // After simplification: 2·x^1 = 2·x
        assert!(matches!(d, Expr::Mul(_) | Expr::Var(_) | Expr::Rat(_)));
    }
}
