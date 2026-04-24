use alloc::boxed::Box;
use alloc::string::String;
use crate::expr::Expr;

/// Substitute `var` with `value` everywhere in `expr`.
pub fn subst(expr: &Expr, var: &str, value: &Expr) -> Expr {
    match expr {
        Expr::Var(name) if name == var => value.clone(),
        Expr::Var(_) | Expr::Rat(_) | Expr::Float(_) | Expr::Const(_) => expr.clone(),
        Expr::Neg(inner)     => Expr::Neg(Box::new(subst(inner, var, value))),
        Expr::Add(terms)     => Expr::Add(terms.iter().map(|t| subst(t, var, value)).collect()),
        Expr::Mul(factors)   => Expr::Mul(factors.iter().map(|f| subst(f, var, value)).collect()),
        Expr::Pow(base, exp) => Expr::Pow(
            Box::new(subst(base, var, value)),
            Box::new(subst(exp, var, value)),
        ),
        Expr::Call(f, args)     => Expr::Call(f.clone(), args.iter().map(|a| subst(a, var, value)).collect()),
        Expr::FnCall(name, args)=> Expr::FnCall(name.clone(), args.iter().map(|a| subst(a, var, value)).collect()),
        Expr::Equation(l, r)    => Expr::Equation(
            Box::new(subst(l, var, value)),
            Box::new(subst(r, var, value)),
        ),
        Expr::Matrix(rows) => Expr::Matrix(
            rows.iter().map(|row| row.iter().map(|e| subst(e, var, value)).collect()).collect()
        ),
        Expr::List(items) => Expr::List(items.iter().map(|e| subst(e, var, value)).collect()),
    }
}

/// Substitute multiple `(name, value)` bindings sequentially.
pub fn subst_all(expr: &Expr, bindings: &[(String, Expr)]) -> Expr {
    let mut result = expr.clone();
    for (var, val) in bindings {
        result = subst(&result, var, val);
    }
    result
}

/// Returns true if `expr` contains the variable named `var`.
pub fn contains_var(expr: &Expr, var: &str) -> bool {
    match expr {
        Expr::Var(name)      => name == var,
        Expr::Rat(_) | Expr::Float(_) | Expr::Const(_) => false,
        Expr::Neg(inner)     => contains_var(inner, var),
        Expr::Add(terms)     => terms.iter().any(|t| contains_var(t, var)),
        Expr::Mul(factors)   => factors.iter().any(|f| contains_var(f, var)),
        Expr::Pow(base, exp) => contains_var(base, var) || contains_var(exp, var),
        Expr::Call(_, args)       => args.iter().any(|a| contains_var(a, var)),
        Expr::FnCall(_, args)     => args.iter().any(|a| contains_var(a, var)),
        Expr::Equation(l, r)      => contains_var(l, var) || contains_var(r, var),
        Expr::Matrix(rows)        => rows.iter().any(|row| row.iter().any(|e| contains_var(e, var))),
        Expr::List(items)         => items.iter().any(|e| contains_var(e, var)),
    }
}
