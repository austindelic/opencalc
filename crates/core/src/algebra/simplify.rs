use crate::env::Env;
use crate::expr::{BuiltinFn, Expr};
use crate::rational::Rational;
use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Algebraically simplify an expression tree (bottom-up).
pub fn simplify(expr: Expr) -> Expr {
    simplify_impl(expr, None)
}

/// Like `simplify` but resolves user-defined functions when encountered in
/// `solve(f, x)` calls, allowing function names to be passed by name.
pub fn simplify_with_env(expr: Expr, env: &Env) -> Expr {
    simplify_impl(expr, Some(env))
}

fn simplify_impl(expr: Expr, env: Option<&Env>) -> Expr {
    let expr = simplify_children(expr, env);
    simplify_node(expr, env)
}

fn simplify_children(expr: Expr, env: Option<&Env>) -> Expr {
    match expr {
        Expr::Neg(inner) => Expr::Neg(Box::new(simplify_impl(*inner, env))),
        Expr::Add(terms) => Expr::Add(terms.into_iter().map(|e| simplify_impl(e, env)).collect()),
        Expr::Mul(factors) => {
            Expr::Mul(factors.into_iter().map(|e| simplify_impl(e, env)).collect())
        }
        Expr::Pow(base, exp) => Expr::Pow(
            Box::new(simplify_impl(*base, env)),
            Box::new(simplify_impl(*exp, env)),
        ),
        Expr::Call(f, args) => {
            Expr::Call(f, args.into_iter().map(|e| simplify_impl(e, env)).collect())
        }
        Expr::FnCall(name, args) => Expr::FnCall(
            name,
            args.into_iter().map(|e| simplify_impl(e, env)).collect(),
        ),
        Expr::Equation(l, r) => Expr::Equation(
            Box::new(simplify_impl(*l, env)),
            Box::new(simplify_impl(*r, env)),
        ),
        Expr::Matrix(rows) => Expr::Matrix(
            rows.into_iter()
                .map(|row| row.into_iter().map(|e| simplify_impl(e, env)).collect())
                .collect(),
        ),
        Expr::List(items) => Expr::List(items.into_iter().map(|e| simplify_impl(e, env)).collect()),
        other => other,
    }
}

fn simplify_node(expr: Expr, env: Option<&Env>) -> Expr {
    match expr {
        Expr::Neg(inner) => simplify_neg(*inner),
        Expr::Add(terms) => simplify_add(terms),
        Expr::Mul(factors) => try_rational_cancel(simplify_mul(factors)),
        Expr::Pow(base, exp) => simplify_pow(*base, *exp),
        Expr::Call(f, args) => simplify_call(f, args, env),
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

    // Canonical ordering so that x*y and y*x produce the same Expr.
    sym_factors.sort_by(|a, b| {
        let ka = alloc::format!("{}", a);
        let kb = alloc::format!("{}", b);
        ka.cmp(&kb)
    });

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

fn simplify_call(f: BuiltinFn, args: Vec<Expr>, env: Option<&Env>) -> Expr {
    // ── Symbolic calculus ops ─────────────────────────────────────────────────
    match (&f, args.as_slice()) {
        // diff(expr, var) → symbolic derivative
        (BuiltinFn::Diff, [expr, Expr::Var(var)]) => {
            return simplify_impl(crate::diff::diff(expr, var), env);
        }
        // diff(expr, var, n) → nth derivative
        (BuiltinFn::Diff, [expr, Expr::Var(var), Expr::Rat(n)])
            if n.is_integer() && n.numer > 0 =>
        {
            let mut result = expr.clone();
            for _ in 0..n.numer {
                result = simplify_impl(crate::diff::diff(&result, var), env);
            }
            return result;
        }
        // integrate(expr, var) → indefinite integral
        (BuiltinFn::Integrate, [expr, Expr::Var(var)]) => {
            if let Some(antideriv) = crate::integrate::integrate(expr, var) {
                return simplify_impl(antideriv, env);
            }
        }
        // taylor(expr, var, point, order)
        (BuiltinFn::Taylor, [expr, Expr::Var(var), point, Expr::Rat(order)])
            if order.is_integer() && order.numer >= 0 =>
        {
            if let Ok(series) = crate::series::taylor(expr, var, point, order.numer as usize) {
                return simplify_impl(series, env);
            }
        }
        // solve(fn_name, var) — resolve user-defined function by name
        (BuiltinFn::Solve, [Expr::Var(fname), Expr::Var(var)]) if fname != var => {
            if let Some(env) = env {
                if let Some(uf) = env.fns.get(fname.as_str()) {
                    if uf.params.len() == 1 {
                        let body =
                            crate::subst::subst(&uf.body, &uf.params[0], &Expr::Var(var.clone()));
                        let target = simplify_impl(body, Some(env));
                        if let Ok(roots) = crate::solve::solve(&target, var) {
                            return roots;
                        }
                    }
                }
            }
        }
        // solve(expr, var) or solve(equation, var)
        (BuiltinFn::Solve, [expr, Expr::Var(var)]) => {
            let target = match expr {
                Expr::Equation(l, r) => simplify_impl(
                    Expr::Add(vec![*l.clone(), Expr::Neg(Box::new(*r.clone()))]),
                    env,
                ),
                other => other.clone(),
            };
            if let Ok(roots) = crate::solve::solve(&target, var) {
                return roots;
            }
        }
        // solve(equation)  — infer variable
        (BuiltinFn::Solve, [Expr::Equation(l, r)]) => {
            let diff = simplify_impl(
                Expr::Add(vec![*l.clone(), Expr::Neg(Box::new(*r.clone()))]),
                env,
            );
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
                return simplify_impl(expand_expr(result), env);
            }
        }
        (BuiltinFn::Tr, [Expr::Matrix(rows)]) => {
            if let Ok(result) = crate::matrix::mat_trace(rows) {
                return simplify_impl(expand_expr(result), env);
            }
        }
        (BuiltinFn::Transpose, [Expr::Matrix(rows)]) => {
            if let Ok(result) = crate::matrix::mat_transpose(rows) {
                return Expr::Matrix(
                    result
                        .into_iter()
                        .map(|row| row.into_iter().map(|e| simplify_impl(e, env)).collect())
                        .collect(),
                );
            }
        }
        (BuiltinFn::Inv, [Expr::Matrix(rows)]) => {
            if let Ok(result) = crate::matrix::mat_inv(rows) {
                return Expr::Matrix(
                    result
                        .into_iter()
                        .map(|row| row.into_iter().map(|e| simplify_impl(e, env)).collect())
                        .collect(),
                );
            }
        }
        (BuiltinFn::Dot, [Expr::List(a), Expr::List(b)]) => {
            if let Ok(result) = crate::matrix::dot(a, b) {
                return simplify_impl(result, env);
            }
        }
        (BuiltinFn::Cross, [Expr::List(a), Expr::List(b)]) => {
            if let Ok(result) = crate::matrix::cross3(a, b) {
                return Expr::List(result.into_iter().map(|e| simplify_impl(e, env)).collect());
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
                        terms[i] = if c.is_one() {
                            Expr::one()
                        } else {
                            Expr::Rat(c)
                        };
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
                        terms[i] = if c.is_one() {
                            Expr::one()
                        } else {
                            Expr::Rat(c)
                        };
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
        Expr::Neg(inner) => {
            // Distribute negation over addition: -(a+b) → -a + -b
            let e = expand_expr(*inner);
            match e {
                Expr::Add(terms) => simplify(Expr::Add(
                    terms
                        .into_iter()
                        .map(|t| expand_expr(Expr::Neg(Box::new(t))))
                        .collect(),
                )),
                other => simplify(Expr::Neg(Box::new(other))),
            }
        }
        Expr::Add(terms) => simplify(Expr::Add(terms.into_iter().map(expand_expr).collect())),
        Expr::Mul(factors) => distribute_mul(factors.into_iter().map(expand_expr).collect()),
        Expr::Pow(base, exp) => {
            let base = expand_expr(*base);
            match *exp {
                Expr::Rat(r) if r.is_integer() && r.numer >= 2 => {
                    // Repeated multiplication with intermediate simplification (avoids
                    // exponential blowup for multivariate polynomials like (x+y+z+1)^10).
                    let mut acc = base.clone();
                    for _ in 1..r.numer {
                        acc = distribute_mul(vec![acc, base.clone()]);
                    }
                    acc
                }
                e => simplify(Expr::Pow(Box::new(base), Box::new(e))),
            }
        }
        other => simplify(other),
    }
}

/// Multiply a list of factors, distributing over Add terms and collecting like
/// terms after every pairwise step to bound intermediate expression size.
fn distribute_mul(factors: Vec<Expr>) -> Expr {
    let mut acc = Expr::one();
    for factor in factors {
        acc = distribute_two(acc, factor);
    }
    acc
}

fn distribute_two(a: Expr, b: Expr) -> Expr {
    match (a, b) {
        (Expr::Add(at), Expr::Add(bt)) => {
            let mut terms = Vec::new();
            for x in &at {
                for y in &bt {
                    terms.push(simplify(Expr::Mul(vec![x.clone(), y.clone()])));
                }
            }
            simplify(Expr::Add(terms))
        }
        (Expr::Add(at), b) => {
            let terms: Vec<Expr> = at
                .into_iter()
                .map(|x| simplify(Expr::Mul(vec![x, b.clone()])))
                .collect();
            simplify(Expr::Add(terms))
        }
        (a, Expr::Add(bt)) => {
            let terms: Vec<Expr> = bt
                .into_iter()
                .map(|y| simplify(Expr::Mul(vec![a.clone(), y])))
                .collect();
            simplify(Expr::Add(terms))
        }
        (a, b) => simplify(Expr::Mul(vec![a, b])),
    }
}

// ── Polynomial rational simplification ────────────────────────────────────────
// Cancels common polynomial factors in rational expressions like (x^6-1)/(x^3-1).

type Poly = Vec<Rational>;

fn poly_trim(mut p: Poly) -> Poly {
    while p.len() > 1 && p.last().map(|r: &Rational| r.is_zero()).unwrap_or(false) {
        p.pop();
    }
    if p.is_empty() {
        vec![Rational::zero()]
    } else {
        p
    }
}

fn poly_is_zero(p: &[Rational]) -> bool {
    p.is_empty() || (p.len() == 1 && p[0].is_zero())
}

fn poly_degree(p: &[Rational]) -> usize {
    let p = poly_trim(p.to_vec());
    if poly_is_zero(&p) {
        0
    } else {
        p.len() - 1
    }
}

fn poly_add_r(mut a: Poly, b: Poly) -> Poly {
    if b.len() > a.len() {
        a.resize(b.len(), Rational::zero());
    }
    for (i, c) in b.into_iter().enumerate() {
        a[i] = a[i] + c;
    }
    poly_trim(a)
}

fn poly_mul_r(a: &[Rational], b: &[Rational]) -> Poly {
    if poly_is_zero(a) || poly_is_zero(b) {
        return vec![Rational::zero()];
    }
    let mut r = vec![Rational::zero(); a.len() + b.len() - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            r[i + j] = r[i + j] + ai * bj;
        }
    }
    poly_trim(r)
}

fn poly_divmod(dividend: Poly, divisor: Poly) -> Option<(Poly, Poly)> {
    let divisor = poly_trim(divisor);
    if poly_is_zero(&divisor) {
        return None;
    }
    let lead_d = *divisor.last().unwrap();
    if lead_d.is_zero() {
        return None;
    }
    let dd = poly_degree(&divisor);
    let mut rem = poly_trim(dividend);
    let qlen = if poly_degree(&rem) >= dd {
        poly_degree(&rem) - dd + 1
    } else {
        0
    };
    let mut quot = vec![Rational::zero(); qlen];
    while !poly_is_zero(&rem) && poly_degree(&rem) >= dd {
        let rd = poly_degree(&rem);
        let c = rem[rd] / lead_d;
        let pos = rd - dd;
        if pos < quot.len() {
            quot[pos] = c;
        }
        for (i, &di) in divisor.iter().enumerate() {
            if pos + i < rem.len() {
                rem[pos + i] = rem[pos + i] - c * di;
            }
        }
        rem = poly_trim(rem);
    }
    Some((poly_trim(quot), rem))
}

fn poly_gcd_r(a: Poly, b: Poly) -> Poly {
    let a = poly_trim(a);
    let b = poly_trim(b);
    if poly_is_zero(&b) {
        return a;
    }
    if poly_is_zero(&a) {
        return b;
    }
    let lead = *b.last().unwrap();
    if lead.is_zero() {
        return vec![Rational::one()];
    }
    let b_monic: Poly = b.iter().map(|c| *c / lead).collect();
    match poly_divmod(a, b_monic.clone()) {
        Some((_, rem)) => poly_gcd_r(b_monic, rem),
        None => vec![Rational::one()],
    }
}

fn poly_to_var_expr(coeffs: Poly, var: Expr) -> Expr {
    let coeffs = poly_trim(coeffs);
    let mut terms: Vec<Expr> = Vec::new();
    for (i, c) in coeffs.into_iter().enumerate() {
        if c.is_zero() {
            continue;
        }
        let term = match i {
            0 => Expr::Rat(c),
            1 => {
                if c.is_one() {
                    var.clone()
                } else {
                    Expr::Mul(vec![Expr::Rat(c), var.clone()])
                }
            }
            _ => {
                let pwr = Expr::Pow(Box::new(var.clone()), Box::new(Expr::integer(i as i64)));
                if c.is_one() {
                    pwr
                } else {
                    Expr::Mul(vec![Expr::Rat(c), pwr])
                }
            }
        };
        terms.push(term);
    }
    match terms.len() {
        0 => Expr::zero(),
        1 => terms.remove(0),
        _ => Expr::Add(terms),
    }
}

fn poly_to_expr(coeffs: Poly, var: &str) -> Expr {
    poly_to_var_expr(coeffs, Expr::Var(var.into()))
}

fn poly_to_trig_expr(coeffs: Poly, u: &Expr) -> Expr {
    let sin_sq = Expr::Pow(
        Box::new(Expr::Call(BuiltinFn::Sin, vec![u.clone()])),
        Box::new(Expr::integer(2)),
    );
    poly_to_var_expr(coeffs, sin_sq)
}

/// Convert an expression to a univariate polynomial in `var`.
/// Returns None if the expression contains other variables or non-polynomial constructs.
fn expr_to_poly(expr: &Expr, var: &str) -> Option<Poly> {
    match expr {
        Expr::Rat(r) => Some(vec![*r]),
        Expr::Var(v) => {
            if v == var {
                Some(vec![Rational::zero(), Rational::one()])
            } else {
                None
            }
        }
        Expr::Neg(inner) => expr_to_poly(inner, var).map(|p| p.into_iter().map(|c| -c).collect()),
        Expr::Add(terms) => {
            let mut r = vec![Rational::zero()];
            for t in terms {
                r = poly_add_r(r, expr_to_poly(t, var)?);
            }
            Some(poly_trim(r))
        }
        Expr::Mul(factors) => {
            let mut r = vec![Rational::one()];
            for f in factors {
                r = poly_mul_r(&r, &expr_to_poly(f, var)?);
            }
            Some(poly_trim(r))
        }
        Expr::Pow(base, exp) => {
            if let Expr::Rat(r) = exp.as_ref() {
                if r.is_integer() && r.numer >= 0 {
                    let p = expr_to_poly(base, var)?;
                    let mut res = vec![Rational::one()];
                    for _ in 0..r.numer {
                        res = poly_mul_r(&res, &p);
                    }
                    return Some(poly_trim(res));
                }
            }
            None
        }
        _ => None,
    }
}

/// Find a single variable that all expressions are polynomials in.
fn find_single_poly_var(exprs: &[&Expr]) -> Option<String> {
    let mut vars: BTreeSet<String> = BTreeSet::new();
    for e in exprs {
        collect_poly_vars(e, &mut vars);
    }
    if vars.len() == 1 {
        vars.into_iter().next()
    } else {
        None
    }
}

fn collect_poly_vars(expr: &Expr, vars: &mut BTreeSet<String>) {
    match expr {
        Expr::Var(v) => {
            vars.insert(v.clone());
        }
        Expr::Rat(_) | Expr::Float(_) | Expr::Const(_) => {}
        Expr::Neg(inner) => collect_poly_vars(inner, vars),
        Expr::Add(terms) => terms.iter().for_each(|t| collect_poly_vars(t, vars)),
        Expr::Mul(factors) => factors.iter().for_each(|f| collect_poly_vars(f, vars)),
        Expr::Pow(base, exp) => {
            if let Expr::Rat(r) = exp.as_ref() {
                if r.is_integer() && r.numer >= 0 {
                    collect_poly_vars(base, vars);
                    return;
                }
            }
            vars.insert("__nonpoly__".to_string());
        }
        _ => {
            vars.insert("__nonpoly__".to_string());
        }
    }
}

/// Convert an expression to polynomial in sin²(u), substituting cos²(u) = 1 - sin²(u).
fn trig_to_poly(expr: &Expr, u: &Expr) -> Option<Poly> {
    let one_minus_t: Poly = vec![Rational::one(), -Rational::one()]; // 1 - t
    match expr {
        Expr::Rat(r) => Some(vec![*r]),
        Expr::Pow(base, exp) => {
            if let Expr::Rat(r) = exp.as_ref() {
                if r.is_integer() && r.numer % 2 == 0 && r.numer >= 2 {
                    let k = r.numer / 2;
                    if let Expr::Call(f, args) = base.as_ref() {
                        if args.len() == 1 && &args[0] == u {
                            if *f == BuiltinFn::Sin {
                                let mut p = vec![Rational::one()];
                                let t = vec![Rational::zero(), Rational::one()];
                                for _ in 0..k {
                                    p = poly_mul_r(&p, &t);
                                }
                                return Some(p);
                            }
                            if *f == BuiltinFn::Cos {
                                let mut p = vec![Rational::one()];
                                for _ in 0..k {
                                    p = poly_mul_r(&p, &one_minus_t);
                                }
                                return Some(p);
                            }
                        }
                    }
                }
            }
            None
        }
        Expr::Neg(inner) => trig_to_poly(inner, u).map(|p| p.into_iter().map(|c| -c).collect()),
        Expr::Add(terms) => {
            let mut r = vec![Rational::zero()];
            for t in terms {
                r = poly_add_r(r, trig_to_poly(t, u)?);
            }
            Some(poly_trim(r))
        }
        Expr::Mul(factors) => {
            let mut r = vec![Rational::one()];
            for f in factors {
                r = poly_mul_r(&r, &trig_to_poly(f, u)?);
            }
            Some(poly_trim(r))
        }
        _ => None,
    }
}

fn find_trig_arg_in(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Pow(base, exp) => {
            if let Expr::Rat(r) = exp.as_ref() {
                if r.is_integer() && r.numer % 2 == 0 && r.numer >= 2 {
                    if let Expr::Call(f, args) = base.as_ref() {
                        if (*f == BuiltinFn::Sin || *f == BuiltinFn::Cos) && args.len() == 1 {
                            return Some(args[0].clone());
                        }
                    }
                }
            }
            find_trig_arg_in(base).or_else(|| find_trig_arg_in(exp))
        }
        Expr::Neg(inner) => find_trig_arg_in(inner),
        Expr::Add(terms) => terms.iter().find_map(find_trig_arg_in),
        Expr::Mul(factors) => factors.iter().find_map(find_trig_arg_in),
        _ => None,
    }
}

/// If `expr` is a Mul containing denominator factors (Pow(_, -1)), try to cancel
/// common polynomial factors between numerator and denominator.
fn try_rational_cancel(expr: Expr) -> Expr {
    let factors = match &expr {
        Expr::Mul(f) => f.clone(),
        _ => return expr,
    };

    let has_denom = factors
        .iter()
        .any(|f| matches!(f, Expr::Pow(_, e) if e.as_ref() == &Expr::neg_one()));
    if !has_denom {
        return expr;
    }

    let mut numer: Vec<Expr> = Vec::new();
    let mut denom: Vec<Expr> = Vec::new();
    for f in &factors {
        if let Expr::Pow(base, exp) = f {
            if exp.as_ref() == &Expr::neg_one() {
                denom.push(*base.clone());
                continue;
            }
        }
        numer.push(f.clone());
    }
    if denom.is_empty() {
        return expr;
    }

    // Try univariate polynomial GCD
    if let Some(r) = cancel_poly(&numer, &denom) {
        return simplify_impl(r, None);
    }

    // Try trig polynomial GCD (handles sin^4-cos^4 / sin^2-cos^2 etc.)
    if let Some(r) = cancel_trig_poly(&numer, &denom) {
        return simplify_impl(r, None);
    }

    expr
}

fn cancel_poly(numer: &[Expr], denom: &[Expr]) -> Option<Expr> {
    let all: Vec<&Expr> = numer.iter().chain(denom.iter()).collect();
    let var = find_single_poly_var(&all)?;
    if var == "__nonpoly__" {
        return None;
    }

    let mut np = vec![Rational::one()];
    for e in numer {
        np = poly_mul_r(&np, &expr_to_poly(e, &var)?);
    }
    let np = poly_trim(np);

    let mut dp = vec![Rational::one()];
    for e in denom {
        dp = poly_mul_r(&dp, &expr_to_poly(e, &var)?);
    }
    let dp = poly_trim(dp);

    let g = poly_gcd_r(np.clone(), dp.clone());
    if poly_degree(&g) == 0 {
        return None;
    }

    let (nq, nr) = poly_divmod(np, g.clone())?;
    let (dq, dr) = poly_divmod(dp, g)?;
    if !poly_is_zero(&nr) || !poly_is_zero(&dr) {
        return None;
    }

    // Normalise: absorb constant denominator into numerator
    let dq = poly_trim(dq);
    let nq = poly_trim(nq);
    let (nq, dq) = if dq.len() == 1 && !dq[0].is_zero() {
        let c = dq[0];
        (
            nq.into_iter().map(|x| x / c).collect::<Poly>(),
            vec![Rational::one()],
        )
    } else {
        (nq, dq)
    };

    let ne = poly_to_expr(poly_trim(nq), &var);
    let de = poly_trim(dq);

    if de == vec![Rational::one()] {
        Some(ne)
    } else {
        Some(Expr::Mul(vec![
            ne,
            Expr::Pow(Box::new(poly_to_expr(de, &var)), Box::new(Expr::neg_one())),
        ]))
    }
}

fn cancel_trig_poly(numer: &[Expr], denom: &[Expr]) -> Option<Expr> {
    let ne = if numer.len() == 1 {
        numer[0].clone()
    } else {
        Expr::Mul(numer.to_vec())
    };
    let de = if denom.len() == 1 {
        denom[0].clone()
    } else {
        Expr::Mul(denom.to_vec())
    };
    let u = find_trig_arg_in(&ne).or_else(|| find_trig_arg_in(&de))?;

    let np = poly_trim(trig_to_poly(&ne, &u)?);
    let dp = poly_trim(trig_to_poly(&de, &u)?);

    let g = poly_gcd_r(np.clone(), dp.clone());
    if poly_degree(&g) == 0 {
        return None;
    }

    let (nq, nr) = poly_divmod(np, g.clone())?;
    let (dq, dr) = poly_divmod(dp, g)?;
    if !poly_is_zero(&nr) || !poly_is_zero(&dr) {
        return None;
    }

    let nq = poly_trim(nq);
    let dq = poly_trim(dq);

    let (nq, dq) = if dq.len() == 1 && !dq[0].is_zero() {
        let c = dq[0];
        (
            nq.into_iter().map(|x| x / c).collect::<Poly>(),
            vec![Rational::one()],
        )
    } else {
        (nq, dq)
    };

    let numer_expr = poly_to_trig_expr(nq, &u);
    let denom_expr = poly_to_trig_expr(dq, &u);

    if denom_expr.is_one() || matches!(&denom_expr, Expr::Rat(r) if r.is_one()) {
        Some(numer_expr)
    } else {
        Some(Expr::Mul(vec![
            numer_expr,
            Expr::Pow(Box::new(denom_expr), Box::new(Expr::neg_one())),
        ]))
    }
}

// ─────────────────────────────────────────────────────────────────────────────

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
        use crate::eval::{eval, Context};
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

    // ── Nightmare cases ───────────────────────────────────────────────────────

    fn eval_at(expr: &Expr, var: &str, val: f64) -> f64 {
        use crate::env::Env;
        use crate::eval::eval_env;
        let mut env = Env::new();
        env.set_var(var, Expr::Float(val));
        eval_env(expr, &env).unwrap_or(f64::NAN)
    }

    #[test]
    fn nightmare_deep_exp_ln() {
        // exp(ln(exp(ln(exp(ln(x^2+1)))))) = x^2+1
        assert_eq!(
            s("simplify(exp(ln(exp(ln(exp(ln(x^2 + 1)))))))"),
            s("x^2 + 1")
        );
    }

    #[test]
    fn nightmare_trig_ratio() {
        // (sin^4 - cos^4)/(sin^2 - cos^2) = 1
        assert_eq!(
            s("simplify((sin(x)^4 - cos(x)^4) / (sin(x)^2 - cos(x)^2))"),
            Expr::one()
        );
    }

    #[test]
    fn nightmare_pythagorean_power_diff() {
        // diff((sin^2+cos^2)^50 * exp(ln(exp(ln(x^2+1)))), x) = 2x
        let d = s("diff((sin(x)^2 + cos(x)^2)^50 * exp(ln(exp(ln(x^2 + 1)))), x)");
        // Evaluate at x=3: should be 6
        assert!((eval_at(&d, "x", 3.0) - 6.0).abs() < 1e-6);
    }

    #[test]
    fn nightmare_ftc_nonlinear_bounds() {
        // d/dx ∫(x²+sin(x) → x³+cos(x)) exp(t³+sin²(t)+cos²(t)) dt
        // = exp((x³+cos(x))³+1)·(3x²-sin(x)) - exp((x²+sin(x))³+1)·(2x+cos(x))
        let d =
            s("diff(integral(exp(t^3 + sin(t)^2 + cos(t)^2), t, x^2 + sin(x), x^3 + cos(x)), x)");
        // Not NaN/infinite at x=0
        let v = eval_at(&d, "x", 0.0);
        assert!(
            v.is_finite(),
            "FTC result should be finite at x=0, got {}",
            v
        );
    }

    #[test]
    fn nightmare_expand_multinomial() {
        // (x+y+z+1)^10 should expand to exactly 286 terms
        let e = s("expand((x + y + z + 1)^10)");
        let terms = match &e {
            Expr::Add(t) => t.len(),
            _ => 1,
        };
        assert_eq!(terms, 286);
    }

    #[test]
    fn nightmare_poly_div_cubic() {
        // (x^6-1)/(x^3-1) = x^3+1; verify numerically at two points
        let d = s("simplify((x^6 - 1) / (x^3 - 1))");
        assert!((eval_at(&d, "x", 2.0) - 9.0).abs() < 1e-9); // 2^3+1 = 9
        assert!((eval_at(&d, "x", 3.0) - 28.0).abs() < 1e-9); // 3^3+1 = 28
    }

    #[test]
    fn nightmare_det_4x4() {
        // det of circulant [[x,1,1,1],...] = (x-1)^3*(x+3) = x^4-6x^2+8x-3
        let d = s("det([[x,1,1,1],[1,x,1,1],[1,1,x,1],[1,1,1,x]])");
        // Numerically verify at x=2: (2-1)^3*(2+3) = 5
        assert!((eval_at(&d, "x", 2.0) - 5.0).abs() < 1e-6);
        // And at x=0: (-1)^3*(3) = -3
        assert!((eval_at(&d, "x", 0.0) - (-3.0)).abs() < 1e-6);
    }

    #[test]
    fn nightmare_nested_integral_ftc() {
        // d/dx ∫(0→x) [∫(0→t) e^(u²) du] dt = ∫(0→x) e^(u²) du
        let d = s("diff(integral(integral(exp(u^2), u, 0, t), t, 0, x), x)");
        // Result should still contain the inner integral (kept symbolic)
        assert!(matches!(d, Expr::Call(BuiltinFn::Integral, _)));
    }

    #[test]
    fn nightmare_poly_div_octic() {
        // (x^8-1)/(x^4-1) = x^4+1; verify numerically
        let d = s("simplify((x^8 - 1) / (x^4 - 1))");
        assert!((eval_at(&d, "x", 2.0) - 17.0).abs() < 1e-9); // 2^4+1 = 17
        assert!((eval_at(&d, "x", 3.0) - 82.0).abs() < 1e-9); // 3^4+1 = 82
    }

    #[test]
    fn nightmare_exp_ln_trig_pythagorean() {
        // exp(ln(sin^2+cos^2)) = exp(ln(1)) = 1
        assert_eq!(s("simplify(exp(ln(sin(x)^2 + cos(x)^2)))"), Expr::one());
    }

    #[test]
    fn nightmare_ln_exp_chain() {
        // ln(exp(ln(exp(ln(exp(x)))))) = x
        assert_eq!(
            s("simplify(ln(exp(ln(exp(ln(exp(x)))))))"),
            Expr::Var("x".into())
        );
    }

    #[test]
    fn nightmare_algebraic_cancel() {
        // (x+1)(x-1)(x^2+1)/(x^4-1) = 1
        assert_eq!(s("simplify((x+1)(x-1)(x^2+1)/(x^4-1))"), Expr::one());
    }

    #[test]
    fn nightmare_det_2x2_symbolic() {
        // det([[x^2+1, x],[x, x^2+1]]) = (x^2+1)^2 - x^2 = x^4+x^2+1
        let d = s("det([[x^2+1, x],[x, x^2+1]])");
        // Verify at x=1: 1+1+1 = 3
        assert!((eval_at(&d, "x", 1.0) - 3.0).abs() < 1e-6);
        // At x=2: 16+4+1 = 21
        assert!((eval_at(&d, "x", 2.0) - 21.0).abs() < 1e-6);
    }
}
