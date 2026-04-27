use crate::rational::Rational;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
    Pi,
    E,
    I,
    Inf,
    NegInf,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuiltinFn {
    // ── Trig ─────────────────────────────────────────────────────────────────
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    // ── Exp / Log ────────────────────────────────────────────────────────────
    Exp,
    Ln,
    Log,
    Log2,
    Log10,
    // ── Roots / Power ────────────────────────────────────────────────────────
    Sqrt,
    Cbrt,
    // ── Rounding / Sign ──────────────────────────────────────────────────────
    Abs,
    Floor,
    Ceil,
    Round,
    Sign,
    // ── Integer / Combinatorics ──────────────────────────────────────────────
    Factorial,
    Gcd,
    Lcm,
    Mod,
    IsPrime,
    // ── Extrema ──────────────────────────────────────────────────────────────
    Max,
    Min,
    // ── Symbolic calculus ────────────────────────────────────────────────────
    Diff,       // diff(expr, var)  or  diff(expr, var, n)
    NDiff,      // ndiff(expr, var, point)   numerical derivative
    Integrate,  // integrate(expr, var)       — indefinite integral
    Integral,   // integral(expr, var, a, b)  — symbolic definite integral (kept symbolic)
    NIntegrate, // nintegrate(expr, var, a, b)
    Solve,      // solve(expr, var)  → List of roots  (expr = 0)
    Taylor,     // taylor(expr, var, point, order)
    Limit,      // limit(expr, var, point)  [stub – returns unevaluated]
    // ── Algebraic manipulation ───────────────────────────────────────────────
    Simplify, // simplify(expr)
    Expand,   // expand(expr)   basic polynomial expansion
    // ── Complex ──────────────────────────────────────────────────────────────
    Re,
    Im,
    Conj,
    Arg,
    // ── Matrix / Vector ──────────────────────────────────────────────────────
    Det,
    Tr,
    Transpose,
    Inv,
    Zeros,
    Ones,
    Eye,
    Dot,
    Cross,
    Norm,
    // ── Sequences / Lists ────────────────────────────────────────────────────
    Range, // range(n)  or  range(start, end)  or  range(start, end, step)
    Len,
    Sum,     // sum(expr, var, from, to)   – numerical summation
    Product, // product(expr, var, from, to)
    // ── Logic ────────────────────────────────────────────────────────────────
    If, // if(cond, then, else)
    // ── Misc ─────────────────────────────────────────────────────────────────
    Random, // random()  or  random(lo, hi)
    Numerator,
    Denominator,
}

impl BuiltinFn {
    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "tan" => Self::Tan,
            "asin" | "arcsin" => Self::Asin,
            "acos" | "arccos" => Self::Acos,
            "atan" | "arctan" => Self::Atan,
            "atan2" => Self::Atan2,
            "sinh" => Self::Sinh,
            "cosh" => Self::Cosh,
            "tanh" => Self::Tanh,
            "asinh" | "arcsinh" => Self::Asinh,
            "acosh" | "arccosh" => Self::Acosh,
            "atanh" | "arctanh" => Self::Atanh,
            "exp" => Self::Exp,
            "ln" => Self::Ln,
            "log" => Self::Log,
            "log2" => Self::Log2,
            "log10" => Self::Log10,
            "sqrt" => Self::Sqrt,
            "cbrt" => Self::Cbrt,
            "abs" => Self::Abs,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "round" => Self::Round,
            "sign" | "sgn" => Self::Sign,
            "factorial" | "fact" => Self::Factorial,
            "gcd" => Self::Gcd,
            "lcm" => Self::Lcm,
            "mod" => Self::Mod,
            "isprime" | "is_prime" => Self::IsPrime,
            "max" => Self::Max,
            "min" => Self::Min,
            "diff" | "derivative" | "D" => Self::Diff,
            "ndiff" => Self::NDiff,
            "integrate" => Self::Integrate,
            "integral" => Self::Integral,
            "nintegrate" => Self::NIntegrate,
            "solve" => Self::Solve,
            "taylor" | "series" => Self::Taylor,
            "limit" | "lim" => Self::Limit,
            "simplify" => Self::Simplify,
            "expand" => Self::Expand,
            "re" | "Re" | "real" => Self::Re,
            "im" | "Im" | "imag" => Self::Im,
            "conj" | "conjugate" => Self::Conj,
            "arg" | "Arg" => Self::Arg,
            "det" => Self::Det,
            "tr" | "trace" => Self::Tr,
            "transpose" | "T" => Self::Transpose,
            "inv" | "inverse" => Self::Inv,
            "zeros" => Self::Zeros,
            "ones" => Self::Ones,
            "eye" | "identity" => Self::Eye,
            "dot" => Self::Dot,
            "cross" => Self::Cross,
            "norm" => Self::Norm,
            "range" => Self::Range,
            "len" | "length" => Self::Len,
            "sum" => Self::Sum,
            "product" | "prod" => Self::Product,
            "if" => Self::If,
            "random" | "rand" => Self::Random,
            "numer" | "numerator" => Self::Numerator,
            "denom" | "denominator" => Self::Denominator,
            _ => return None,
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Atan2 => "atan2",
            Self::Sinh => "sinh",
            Self::Cosh => "cosh",
            Self::Tanh => "tanh",
            Self::Asinh => "asinh",
            Self::Acosh => "acosh",
            Self::Atanh => "atanh",
            Self::Exp => "exp",
            Self::Ln => "ln",
            Self::Log => "log",
            Self::Log2 => "log2",
            Self::Log10 => "log10",
            Self::Sqrt => "sqrt",
            Self::Cbrt => "cbrt",
            Self::Abs => "abs",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Round => "round",
            Self::Sign => "sign",
            Self::Factorial => "factorial",
            Self::Gcd => "gcd",
            Self::Lcm => "lcm",
            Self::Mod => "mod",
            Self::IsPrime => "isprime",
            Self::Max => "max",
            Self::Min => "min",
            Self::Diff => "diff",
            Self::NDiff => "ndiff",
            Self::Integrate => "integrate",
            Self::Integral => "integral",
            Self::NIntegrate => "nintegrate",
            Self::Solve => "solve",
            Self::Taylor => "taylor",
            Self::Limit => "limit",
            Self::Simplify => "simplify",
            Self::Expand => "expand",
            Self::Re => "re",
            Self::Im => "im",
            Self::Conj => "conj",
            Self::Arg => "arg",
            Self::Det => "det",
            Self::Tr => "tr",
            Self::Transpose => "transpose",
            Self::Inv => "inv",
            Self::Zeros => "zeros",
            Self::Ones => "ones",
            Self::Eye => "eye",
            Self::Dot => "dot",
            Self::Cross => "cross",
            Self::Norm => "norm",
            Self::Range => "range",
            Self::Len => "len",
            Self::Sum => "sum",
            Self::Product => "product",
            Self::If => "if",
            Self::Random => "random",
            Self::Numerator => "numer",
            Self::Denominator => "denom",
        }
    }
}

/// Symbolic expression tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// Exact rational number
    Rat(Rational),
    /// IEEE-754 float literal
    Float(f64),
    /// Named mathematical constant
    Const(Constant),
    /// Symbolic variable
    Var(String),
    /// Negation: -x
    Neg(Box<Expr>),
    /// N-ary addition
    Add(Vec<Expr>),
    /// N-ary multiplication
    Mul(Vec<Expr>),
    /// Exponentiation: base ^ exp
    Pow(Box<Expr>, Box<Expr>),
    /// Builtin function call
    Call(BuiltinFn, Vec<Expr>),
    /// User-defined function call
    FnCall(String, Vec<Expr>),
    /// Equation: lhs == rhs  (used as argument to solve())
    Equation(Box<Expr>, Box<Expr>),
    /// 2-D symbolic matrix  (rows × cols)
    Matrix(Vec<Vec<Expr>>),
    /// Ordered list / vector
    List(Vec<Expr>),
}

// ── Constructors ─────────────────────────────────────────────────────────────

impl Expr {
    pub fn zero() -> Self {
        Expr::Rat(Rational::zero())
    }
    pub fn one() -> Self {
        Expr::Rat(Rational::one())
    }
    pub fn neg_one() -> Self {
        Expr::Rat(Rational::neg_one())
    }
    pub fn integer(n: i64) -> Self {
        Expr::Rat(Rational::from(n))
    }
    pub fn rational(n: i64, d: i64) -> Self {
        Expr::Rat(Rational::new(n, d))
    }
    pub fn imag() -> Self {
        Expr::Const(Constant::I)
    }

    pub fn is_zero(&self) -> bool {
        matches!(self, Expr::Rat(r) if r.is_zero())
    }
    pub fn is_one(&self) -> bool {
        matches!(self, Expr::Rat(r) if r.is_one())
    }
    pub fn is_neg_one(&self) -> bool {
        matches!(self, Expr::Rat(r) if r.is_neg_one())
    }

    pub fn neg(self) -> Self {
        Expr::Neg(Box::new(self))
    }

    pub fn add(a: Self, b: Self) -> Self {
        Expr::Add(vec![a, b])
    }
    pub fn sub(a: Self, b: Self) -> Self {
        Expr::Add(vec![a, b.neg()])
    }
    pub fn mul(a: Self, b: Self) -> Self {
        Expr::Mul(vec![a, b])
    }
    pub fn div(a: Self, b: Self) -> Self {
        Expr::Mul(vec![a, Expr::Pow(Box::new(b), Box::new(Expr::neg_one()))])
    }
    pub fn pow(base: Self, exp: Self) -> Self {
        Expr::Pow(Box::new(base), Box::new(exp))
    }

    /// True when this expression contains no variables or unevaluated calls.
    pub fn is_numeric(&self) -> bool {
        match self {
            Expr::Rat(_) | Expr::Float(_) => true,
            Expr::Const(Constant::Pi) | Expr::Const(Constant::E) => true,
            Expr::Const(Constant::Inf) | Expr::Const(Constant::NegInf) => true,
            Expr::Neg(inner) => inner.is_numeric(),
            Expr::Add(ts) => ts.iter().all(|t| t.is_numeric()),
            Expr::Mul(fs) => fs.iter().all(|f| f.is_numeric()),
            Expr::Pow(b, e) => b.is_numeric() && e.is_numeric(),
            Expr::Call(_, args) => args.iter().all(|a| a.is_numeric()),
            _ => false,
        }
    }
}

impl From<i64> for Expr {
    fn from(n: i64) -> Self {
        Expr::Rat(Rational::from(n))
    }
}
impl From<i32> for Expr {
    fn from(n: i32) -> Self {
        Expr::Rat(Rational::from(n))
    }
}
impl From<Rational> for Expr {
    fn from(r: Rational) -> Self {
        Expr::Rat(r)
    }
}

// ── Display ──────────────────────────────────────────────────────────────────

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Pi => write!(f, "π"),
            Constant::E => write!(f, "e"),
            Constant::I => write!(f, "i"),
            Constant::Inf => write!(f, "∞"),
            Constant::NegInf => write!(f, "-∞"),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Rat(r) => write!(f, "{}", r),
            Expr::Float(v) => write!(f, "{}", v),
            Expr::Const(c) => write!(f, "{}", c),
            Expr::Var(s) => write!(f, "{}", s),
            Expr::Neg(inner) => match inner.as_ref() {
                e @ (Expr::Add(_) | Expr::Mul(_)) => write!(f, "-({})", e),
                e => write!(f, "-{}", e),
            },
            Expr::Add(terms) => {
                for (i, term) in terms.iter().enumerate() {
                    if i == 0 {
                        write!(f, "{}", term)?;
                    } else if let Some(pos) = display_negative(term) {
                        // a + -b  →  a - b
                        write!(f, " - {}", paren_add(&pos))?;
                    } else {
                        write!(f, " + {}", term)?;
                    }
                }
                Ok(())
            }
            Expr::Mul(factors) => {
                // Split into numerator factors and denominator (x^-1) factors
                let mut numer: Vec<&Expr> = Vec::new();
                let mut denom: Vec<&Expr> = Vec::new();
                for fac in factors {
                    if let Expr::Pow(base, exp) = fac {
                        if exp.as_ref() == &Expr::neg_one() {
                            denom.push(base.as_ref());
                            continue;
                        }
                    }
                    numer.push(fac);
                }

                // Write numerator
                if numer.is_empty() {
                    write!(f, "1")?;
                } else {
                    for (i, fac) in numer.iter().enumerate() {
                        if i > 0 {
                            write!(f, "·")?;
                        }
                        match fac {
                            Expr::Add(_) => write!(f, "({})", fac)?,
                            // negative factor mid-mul needs parens: 2·(-x)
                            _ if i > 0 && display_negative(fac).is_some() => {
                                write!(f, "({})", fac)?
                            }
                            _ => write!(f, "{}", fac)?,
                        }
                    }
                }

                if !denom.is_empty() {
                    write!(f, "/")?;
                    if denom.len() == 1 {
                        let d = denom[0];
                        match d {
                            Expr::Add(_) | Expr::Mul(_) => write!(f, "({})", d)?,
                            _ => write!(f, "{}", d)?,
                        }
                    } else {
                        write!(f, "(")?;
                        for (i, d) in denom.iter().enumerate() {
                            if i > 0 {
                                write!(f, "·")?;
                            }
                            match d {
                                Expr::Add(_) => write!(f, "({})", d)?,
                                _ => write!(f, "{}", d)?,
                            }
                        }
                        write!(f, ")")?;
                    }
                }
                Ok(())
            }
            Expr::Pow(base, exp) => {
                // x^-1  →  1/x  (or 1/(x) if base needs parens)
                if exp.as_ref() == &Expr::neg_one() {
                    match base.as_ref() {
                        e @ (Expr::Add(_) | Expr::Mul(_)) => return write!(f, "1/({})", e),
                        e => return write!(f, "1/{}", e),
                    }
                }
                let bp = matches!(base.as_ref(), Expr::Add(_) | Expr::Mul(_) | Expr::Neg(_));
                let ep = matches!(
                    exp.as_ref(),
                    Expr::Add(_) | Expr::Mul(_) | Expr::Neg(_) | Expr::Pow(..)
                );
                if bp {
                    write!(f, "({})", base)?;
                } else {
                    write!(f, "{}", base)?;
                }
                write!(f, "^")?;
                if ep {
                    write!(f, "({})", exp)?;
                } else {
                    write!(f, "{}", exp)?;
                }
                Ok(())
            }
            Expr::Call(func, args) => {
                // exp(x)  →  e^x
                if *func == BuiltinFn::Exp && args.len() == 1 {
                    let arg = &args[0];
                    return match arg {
                        Expr::Add(_) | Expr::Mul(_) | Expr::Pow(..) | Expr::Neg(_) => {
                            write!(f, "e^({})", arg)
                        }
                        _ => write!(f, "e^{}", arg),
                    };
                }
                write!(f, "{}(", func.name())?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }
            Expr::FnCall(name, args) => {
                write!(f, "{}(", name)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }
            Expr::Equation(l, r) => write!(f, "{} == {}", l, r),
            Expr::Matrix(rows) => {
                write!(f, "[")?;
                for (i, row) in rows.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "[")?;
                    for (j, e) in row.iter().enumerate() {
                        if j > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", e)?;
                    }
                    write!(f, "]")?;
                }
                write!(f, "]")
            }
            Expr::List(items) => {
                write!(f, "[")?;
                for (i, e) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            }
        }
    }
}

fn paren_add(e: &Expr) -> alloc::string::String {
    if matches!(e, Expr::Add(_)) {
        format!("({})", e)
    } else {
        format!("{}", e)
    }
}

/// If `e` should be displayed with a leading minus in an Add context,
/// returns the positive version. Otherwise returns None.
fn display_negative(e: &Expr) -> Option<Expr> {
    match e {
        Expr::Neg(inner) => Some(*inner.clone()),
        Expr::Rat(r) if r.is_negative() => Some(Expr::Rat(Rational {
            numer: r.numer.abs(),
            denom: r.denom,
        })),
        Expr::Float(v) if *v < 0.0 => Some(Expr::Float(-v)),
        Expr::Mul(factors) if !factors.is_empty() => {
            display_negative(&factors[0]).map(|pos_first| {
                let mut new_factors = factors.clone();
                if pos_first.is_one() {
                    new_factors.remove(0);
                    if new_factors.len() == 1 {
                        new_factors.remove(0)
                    } else {
                        Expr::Mul(new_factors)
                    }
                } else {
                    new_factors[0] = pos_first;
                    Expr::Mul(new_factors)
                }
            })
        }
        _ => None,
    }
}
