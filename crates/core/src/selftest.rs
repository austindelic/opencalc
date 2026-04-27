use crate::lexer::{Lexer, Token};
use crate::rational::Rational;
use crate::tests::CALCULATOR_CONFORMANCE_CASES;
use crate::{diff, eval, eval_env, parse, parse_statement, simplify, Context, Env, Expr};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub struct SelfTestReport {
    pub total: usize,
    pub passed: usize,
    pub failures: Vec<String>,
}

pub fn run_all() -> SelfTestReport {
    let mut report = SelfTestReport {
        total: 0,
        passed: 0,
        failures: Vec::new(),
    };

    for (index, expr) in CALCULATOR_CONFORMANCE_CASES.iter().enumerate() {
        let ok = parse(expr).map(simplify).is_ok();
        record(&mut report, format!("conformance[{index}]"), ok);
    }

    record(
        &mut report,
        "lexer_basic_tokens".into(),
        lexer_basic_tokens(),
    );
    record(
        &mut report,
        "lexer_scientific_notation".into(),
        lexer_scientific_notation(),
    );
    record(
        &mut report,
        "parser_simple_arith".into(),
        parser_simple_arith(),
    );
    record(
        &mut report,
        "parser_power_right_assoc".into(),
        parser_power_right_assoc(),
    );
    record(
        &mut report,
        "parser_implicit_multiply".into(),
        parser_implicit_multiply(),
    );
    record(
        &mut report,
        "parser_list_matrix".into(),
        parser_list_matrix(),
    );
    record(
        &mut report,
        "eval_basic_arithmetic".into(),
        eval_basic_arithmetic(),
    );
    record(&mut report, "eval_constants".into(), eval_constants());
    record(&mut report, "eval_trig".into(), eval_trig());
    record(&mut report, "eval_user_fn".into(), eval_user_fn());
    record(
        &mut report,
        "rational_reduction".into(),
        rational_reduction(),
    );
    record(&mut report, "rational_power".into(), rational_power());
    record(&mut report, "diff_polynomial".into(), diff_polynomial());
    record(&mut report, "diff_trig".into(), diff_trig());
    record(
        &mut report,
        "simplify_like_terms".into(),
        simplify_like_terms(),
    );
    record(
        &mut report,
        "simplify_pythagorean".into(),
        simplify_pythagorean(),
    );
    record(
        &mut report,
        "simplify_expand_square".into(),
        simplify_expand_square(),
    );
    record(
        &mut report,
        "simplify_nightmare_poly_div".into(),
        simplify_nightmare_poly_div(),
    );
    record(
        &mut report,
        "constructed_matches_parser".into(),
        constructed_matches_parser(),
    );

    #[cfg(feature = "scripting")]
    scripting_tests(&mut report);

    report
}

fn record(report: &mut SelfTestReport, name: String, ok: bool) {
    report.total += 1;
    if ok {
        report.passed += 1;
    } else {
        report.failures.push(name);
    }
}

fn lexer_basic_tokens() -> bool {
    Lexer::tokenize("1 + 2.5 * x").ok()
        == Some(vec![
            Token::Number(1.0),
            Token::Plus,
            Token::Number(2.5),
            Token::Star,
            Token::Ident("x".into()),
            Token::Eof,
        ])
}

fn lexer_scientific_notation() -> bool {
    matches!(Lexer::tokenize("1e3").ok(), Some(tokens) if tokens.first() == Some(&Token::Number(1000.0)))
}

fn parser_simple_arith() -> bool {
    parse("1 + 2").ok() == Some(Expr::Add(vec![Expr::integer(1), Expr::integer(2)]))
}

fn parser_power_right_assoc() -> bool {
    parse("2^3^4").ok()
        == Some(Expr::Pow(
            Box::new(Expr::integer(2)),
            Box::new(Expr::Pow(
                Box::new(Expr::integer(3)),
                Box::new(Expr::integer(4)),
            )),
        ))
}

fn parser_implicit_multiply() -> bool {
    parse("2(x+1)").is_ok() && parse("2x").is_ok() && parse("(x+1)(x+2)").is_ok()
}

fn parser_list_matrix() -> bool {
    matches!(parse("[1, 2, 3]").ok(), Some(Expr::List(_)))
        && matches!(parse("[[1, 2], [3, 4]]").ok(), Some(Expr::Matrix(_)))
        && parse_statement("x = 2").is_ok()
}

fn eval_basic_arithmetic() -> bool {
    approx(eval_src("1 + 2"), 3.0)
        && approx(eval_src("10 - 3"), 7.0)
        && approx(eval_src("4 * 5"), 20.0)
        && approx(eval_src("10 / 4"), 2.5)
        && approx(eval_src("5!"), 120.0)
        && approx(eval_src("10 % 3"), 1.0)
}

fn eval_constants() -> bool {
    approx(eval_src("pi"), core::f64::consts::PI) && approx(eval_src("e"), core::f64::consts::E)
}

fn eval_trig() -> bool {
    approx(eval_src("sin(0)"), 0.0) && approx(eval_src("cos(0)"), 1.0)
}

fn eval_user_fn() -> bool {
    let mut env = Env::new();
    env.set_fn(
        "square",
        crate::env::UserFn {
            params: alloc::vec!["x".into()],
            body: parse("x^2").unwrap(),
        },
    );
    let expr = parse("square(3)").unwrap();
    approx(eval_env(&expr, &env).unwrap_or(f64::NAN), 9.0)
}

fn rational_reduction() -> bool {
    Rational::new(2, 4) == Rational::new(1, 2)
        && Rational::new(-2, -4) == Rational::new(1, 2)
        && Rational::new(0, 9) == Rational::zero()
}

fn rational_power() -> bool {
    Rational::new(2, 3).checked_pow_int(3) == Some(Rational::new(8, 27))
}

fn diff_polynomial() -> bool {
    let expr = parse("x^2").unwrap();
    let derivative = simplify(diff(&expr, "x"));
    let mut ctx = Context::new();
    ctx.set("x", 3.0);
    approx(eval(&derivative, &ctx).unwrap_or(f64::NAN), 6.0)
}

fn diff_trig() -> bool {
    let expr = parse("sin(x)").unwrap();
    let derivative = simplify(diff(&expr, "x"));
    let mut ctx = Context::new();
    ctx.set("x", 0.0);
    approx(eval(&derivative, &ctx).unwrap_or(f64::NAN), 1.0)
}

fn simplify_like_terms() -> bool {
    simplify(parse("2*x + 3*x").unwrap())
        == Expr::Mul(vec![Expr::integer(5), Expr::Var("x".into())])
}

fn simplify_pythagorean() -> bool {
    simplify(parse("sin(x)^2 + cos(x)^2").unwrap()) == Expr::one()
}

fn simplify_expand_square() -> bool {
    let expanded = simplify(parse("expand((x+1)^2)").unwrap());
    let mut ctx = Context::new();
    ctx.set("x", 3.0);
    approx(eval(&expanded, &ctx).unwrap_or(f64::NAN), 16.0)
}

fn simplify_nightmare_poly_div() -> bool {
    let simplified = simplify(parse("simplify((x^8 - 1) / (x^4 - 1))").unwrap());
    let mut ctx = Context::new();
    ctx.set("x", 2.0);
    approx(eval(&simplified, &ctx).unwrap_or(f64::NAN), 17.0)
}

fn constructed_matches_parser() -> bool {
    let parsed = simplify(parse("sin(pi/6)").unwrap());
    let built = simplify(Expr::Call(
        crate::expr::BuiltinFn::Sin,
        vec![Expr::div(
            Expr::Const(crate::expr::Constant::Pi),
            Expr::integer(6),
        )],
    ));
    parsed == built
}

#[cfg(feature = "scripting")]
fn scripting_tests(report: &mut SelfTestReport) {
    use crate::scripting::ScriptRuntime;

    let runtime = ScriptRuntime::new();
    record(
        report,
        "script_calc_bridge".into(),
        runtime.run(r#"calc("2^10") == "1024""#).ok().as_deref() == Some("true"),
    );
    let compiled = runtime
        .compile(r#"counter += 1; calc("sqrt(144)") + ":" + counter"#)
        .ok();
    let mut scope = ScriptRuntime::new_scope();
    scope.push("counter", 0_i64);
    let compiled_ok = compiled
        .as_ref()
        .and_then(|script| runtime.run_compiled_with_scope(script, &mut scope).ok())
        .as_deref()
        == Some("12:1")
        && compiled
            .as_ref()
            .and_then(|script| runtime.run_compiled_with_scope(script, &mut scope).ok())
            .as_deref()
            == Some("12:2");
    record(report, "script_compiled_scope".into(), compiled_ok);
    record(
        report,
        "script_parse_error".into(),
        matches!(runtime.run(r#"calc("1 +")"#).ok(), Some(result) if result.starts_with("error:")),
    );
    record(
        report,
        "script_nan".into(),
        runtime.run(r#"value("x + 1").is_nan"#).ok().as_deref() == Some("true"),
    );
    record(
        report,
        "script_loop".into(),
        runtime
            .run(
                r#"
                fn total_power(limit) {
                    let sum = 0;
                    for n in 1..=limit {
                        sum += value("2^" + n);
                    }
                    sum
                }
                total_power(4)
                "#,
            )
            .ok()
            .as_deref()
            == Some("30.0"),
    );
}

fn eval_src(src: &str) -> f64 {
    let ctx = Context::new();
    eval(&simplify(parse(src).unwrap()), &ctx).unwrap_or(f64::NAN)
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-10
}
