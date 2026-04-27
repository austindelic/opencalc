use crate::{parse, simplify, CalcError, Expr};

/// Expressions shared by core conformance tests and embedded stress runs.
pub const CALCULATOR_CONFORMANCE_CASES: &[&str] = &[
    "1 + 2 + 3 + 4 + 5",
    "100 / 7 + 3/14",
    "2^10",
    "2^32",
    "1000000 * 999999",
    "2x",
    "3pi",
    "sin(pi/6)",
    "cos(0)",
    "tan(pi/4)",
    "asin(1)",
    "acos(0)",
    "atan(1)",
    "sinh(0)",
    "cosh(0)",
    "tanh(0)",
    "exp(0)",
    "ln(1)",
    "log(10, 100)",
    "log2(8)",
    "log10(1000)",
    "sqrt(144)",
    "cbrt(27)",
    "sin(x)^2 + cos(x)^2",
    "exp(ln(x))",
    "ln(exp(x))",
    "sqrt(x^2)",
    "expand((x+1)^3)",
    "expand((x+2)(x-2))",
    "expand((x+y)^2)",
    "diff(x^3, x)",
    "diff(sin(x), x)",
    "diff(exp(x), x)",
    "diff(ln(x), x)",
    "diff(x^3, x, 2)",
    "integrate(x^2, x)",
    "integrate(sin(x), x)",
    "integrate(exp(x), x)",
    "solve(x^2 - 4, x)",
    "solve(2x + 6, x)",
    "solve(x^2 == 9, x)",
    "taylor(sin(x), x, 0, 5)",
    "taylor(exp(x), x, 0, 4)",
    "gcd(48, 18)",
    "lcm(12, 15)",
    "5!",
    "isprime(97)",
    "sum(x, x, 1, 10)",
    "product(x, x, 1, 5)",
    "range(8)",
    "zeros(3)",
    "ones(2)",
    "eye(3)",
    "det([[1,2],[3,4]])",
    "tr([[1,0,0],[0,2,0],[0,0,3]])",
    "transpose([[1,2,3],[4,5,6]])",
    "dot([3,4], [3,4])",
    "norm([3,4])",
    "re(3 + 4i)",
    "im(3 + 4i)",
    "diff(expand((x^2 + 2x + 1)), x)",
    "simplify(sin(x)^2 + cos(x)^2 + 1)",
    "diff(integral(exp(t^2), t, 1, x), x)",
    "diff(ln(integral(exp(t^2), t, 1, x)), x) * integral(exp(t^2), t, 1, x)",
    "diff(integral(sin(t), t, 0, x), x)",
    "diff(integral(t^2, t, x, 5), x)",
    "diff(integral(exp(t), t, x^2, x^3), x)",
    "diff(integral(cos(t^2), t, 0, x^2), x)",
    "diff(ln(integral(t^3 + 2t, t, 1, x)), x)",
    "diff(integral(exp(t^2), t, 1, x)^2, x)",
    "simplify(exp(ln(exp(ln(exp(ln(x^2 + 1)))))))",
    "simplify((sin(x)^4 - cos(x)^4) / (sin(x)^2 - cos(x)^2))",
    "diff((sin(x)^2 + cos(x)^2)^50 * exp(ln(exp(ln(x^2 + 1)))), x)",
    "diff(integral(exp(t^3 + sin(t)^2 + cos(t)^2), t, x^2 + sin(x), x^3 + cos(x)), x)",
    "diff(ln(integral(exp(t^2 + ln(t^2 + 1)^2), t, sin(x)^2, cos(x)^2 + x)), x)",
    "diff((x^2 + 1) * ln(integral(exp(t^2 + sin(t)^2 + cos(t)^2), t, x, x^2 + 1))^2, x)",
    "simplify((x^6 - 1) / (x^3 - 1))",
    "det([[x,1,1,1],[1,x,1,1],[1,1,x,1],[1,1,1,x]])",
    "diff(exp(x^2) * sin(x)^3, x, 4)",
    "diff(integral(integral(exp(u^2), u, 0, t), t, 0, x), x)",
    "simplify((x^8 - 1) / (x^4 - 1))",
    "simplify(exp(ln(sin(x)^2 + cos(x)^2)))",
    "diff(exp(sin(exp(x^2 + ln(x^2 + 1)))), x)",
    "simplify(ln(exp(ln(exp(ln(exp(x)))))))",
    "diff(x^x^x, x)",
    "simplify((x+1)(x-1)(x^2+1)/(x^4-1))",
    "det([[x^2+1, x],[x, x^2+1]])",
    "diff((sin(x)^2 + cos(x)^2)^5 * exp(ln(x^2 + 1)) * integral(exp(t^2 + sin(t)^2 + cos(t)^2), t, x^2, ln(x + 1)), x)",
];

/// Run the parser-facing calculator path for one conformance expression.
pub fn simplify_parsed_conformance_case(src: &str) -> Result<Expr, CalcError> {
    parse(src).map(simplify)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::{eval, Context};
    use crate::expr::{BuiltinFn, Constant};
    use alloc::boxed::Box;
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;

    struct ConstructedExpressionCase {
        source: &'static str,
        expr: Expr,
        variables: &'static [(&'static str, f64)],
        expected_value: Option<f64>,
    }

    fn var(name: &str) -> Expr {
        Expr::Var(name.to_string())
    }

    fn call(function: BuiltinFn, args: Vec<Expr>) -> Expr {
        Expr::Call(function, args)
    }

    fn matrix_2x2(rows: [[i64; 2]; 2]) -> Expr {
        Expr::Matrix(
            rows.iter()
                .map(|row| row.iter().copied().map(Expr::integer).collect())
                .collect(),
        )
    }

    fn constructed_expression_cases() -> Vec<ConstructedExpressionCase> {
        vec![
            ConstructedExpressionCase {
                source: "1 + 2 + 3 + 4 + 5",
                expr: Expr::Add((1..=5).map(Expr::integer).collect()),
                variables: &[],
                expected_value: Some(15.0),
            },
            ConstructedExpressionCase {
                source: "100 / 7 + 3/14",
                expr: Expr::Add(vec![
                    Expr::div(Expr::integer(100), Expr::integer(7)),
                    Expr::div(Expr::integer(3), Expr::integer(14)),
                ]),
                variables: &[],
                expected_value: Some(14.5),
            },
            ConstructedExpressionCase {
                source: "2x",
                expr: Expr::Mul(vec![Expr::integer(2), var("x")]),
                variables: &[("x", 3.0)],
                expected_value: Some(6.0),
            },
            ConstructedExpressionCase {
                source: "sin(pi/6)",
                expr: call(
                    BuiltinFn::Sin,
                    vec![Expr::div(Expr::Const(Constant::Pi), Expr::integer(6))],
                ),
                variables: &[],
                expected_value: Some(0.5),
            },
            ConstructedExpressionCase {
                source: "exp(ln(x))",
                expr: call(BuiltinFn::Exp, vec![call(BuiltinFn::Ln, vec![var("x")])]),
                variables: &[("x", 7.0)],
                expected_value: Some(7.0),
            },
            ConstructedExpressionCase {
                source: "sqrt(x^2)",
                expr: call(BuiltinFn::Sqrt, vec![Expr::pow(var("x"), Expr::integer(2))]),
                variables: &[("x", 5.0)],
                expected_value: Some(5.0),
            },
            ConstructedExpressionCase {
                source: "expand((x+2)(x-2))",
                expr: call(
                    BuiltinFn::Expand,
                    vec![Expr::Mul(vec![
                        Expr::Add(vec![var("x"), Expr::integer(2)]),
                        Expr::Add(vec![var("x"), Expr::integer(-2)]),
                    ])],
                ),
                variables: &[("x", 9.0)],
                expected_value: Some(77.0),
            },
            ConstructedExpressionCase {
                source: "diff(x^3, x, 2)",
                expr: call(
                    BuiltinFn::Diff,
                    vec![
                        Expr::pow(var("x"), Expr::integer(3)),
                        var("x"),
                        Expr::integer(2),
                    ],
                ),
                variables: &[("x", 4.0)],
                expected_value: Some(24.0),
            },
            ConstructedExpressionCase {
                source: "integrate(x^2, x)",
                expr: call(
                    BuiltinFn::Integrate,
                    vec![Expr::pow(var("x"), Expr::integer(2)), var("x")],
                ),
                variables: &[("x", 3.0)],
                expected_value: Some(9.0),
            },
            ConstructedExpressionCase {
                source: "solve(x^2 == 9, x)",
                expr: call(
                    BuiltinFn::Solve,
                    vec![
                        Expr::Equation(
                            Box::new(Expr::pow(var("x"), Expr::integer(2))),
                            Box::new(Expr::integer(9)),
                        ),
                        var("x"),
                    ],
                ),
                variables: &[],
                expected_value: None,
            },
            ConstructedExpressionCase {
                source: "taylor(exp(x), x, 0, 4)",
                expr: call(
                    BuiltinFn::Taylor,
                    vec![
                        call(BuiltinFn::Exp, vec![var("x")]),
                        var("x"),
                        Expr::zero(),
                        Expr::integer(4),
                    ],
                ),
                variables: &[("x", 1.0)],
                expected_value: Some(2.708333333333333),
            },
            ConstructedExpressionCase {
                source: "sum(x, x, 1, 10)",
                expr: call(
                    BuiltinFn::Sum,
                    vec![var("x"), var("x"), Expr::integer(1), Expr::integer(10)],
                ),
                variables: &[],
                expected_value: Some(55.0),
            },
            ConstructedExpressionCase {
                source: "range(8)",
                expr: call(BuiltinFn::Range, vec![Expr::integer(8)]),
                variables: &[],
                expected_value: None,
            },
            ConstructedExpressionCase {
                source: "zeros(3)",
                expr: call(BuiltinFn::Zeros, vec![Expr::integer(3)]),
                variables: &[],
                expected_value: None,
            },
            ConstructedExpressionCase {
                source: "det([[1,2],[3,4]])",
                expr: call(BuiltinFn::Det, vec![matrix_2x2([[1, 2], [3, 4]])]),
                variables: &[],
                expected_value: Some(-2.0),
            },
            ConstructedExpressionCase {
                source: "diff(integral(sin(t), t, 0, x), x)",
                expr: call(
                    BuiltinFn::Diff,
                    vec![
                        call(
                            BuiltinFn::Integral,
                            vec![
                                call(BuiltinFn::Sin, vec![var("t")]),
                                var("t"),
                                Expr::zero(),
                                var("x"),
                            ],
                        ),
                        var("x"),
                    ],
                ),
                variables: &[("x", 1.2)],
                expected_value: Some(libm::sin(1.2)),
            },
            ConstructedExpressionCase {
                source: "simplify(sin(x)^2 + cos(x)^2 + 1)",
                expr: call(
                    BuiltinFn::Simplify,
                    vec![Expr::Add(vec![
                        Expr::pow(call(BuiltinFn::Sin, vec![var("x")]), Expr::integer(2)),
                        Expr::pow(call(BuiltinFn::Cos, vec![var("x")]), Expr::integer(2)),
                        Expr::one(),
                    ])],
                ),
                variables: &[],
                expected_value: Some(2.0),
            },
        ]
    }

    fn eval_with_variables(expr: &Expr, variables: &[(&str, f64)]) -> f64 {
        let mut ctx = Context::new();
        for (name, value) in variables {
            ctx.set(name, *value);
        }
        eval(expr, &ctx).unwrap()
    }

    #[test]
    fn parser_conformance_cases_should_parse_and_simplify() {
        for case in CALCULATOR_CONFORMANCE_CASES {
            let expr = simplify_parsed_conformance_case(case).unwrap();
            core::hint::black_box(expr);
        }
    }

    #[test]
    fn constructed_expressions_should_match_parser_conformance_cases() {
        for case in constructed_expression_cases() {
            let parsed = simplify_parsed_conformance_case(case.source).unwrap();
            let constructed = simplify(case.expr);
            assert_eq!(constructed, parsed, "mismatch for {}", case.source);

            if let Some(expected) = case.expected_value {
                let actual = eval_with_variables(&constructed, case.variables);
                assert!(
                    (actual - expected).abs() < 1e-9,
                    "bad eval for {}: got {}, expected {}",
                    case.source,
                    actual,
                    expected
                );
            }
        }
    }
}
