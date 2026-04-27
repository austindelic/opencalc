use crate::{
    eval_env, parse_statement, simplify_with_env, CalcError, Env, Expr, Statement, UserFn,
};
use alloc::format;
use alloc::string::{String, ToString};
use rhai::{Dynamic, Engine, Scope, AST};

pub type ScriptScope = Scope<'static>;

pub struct CompiledScript {
    ast: AST,
}

pub struct ScriptRuntime {
    engine: Engine,
}

impl ScriptRuntime {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine.register_fn("calc", calc_text);
        engine.register_fn("simplify", simplify_text);
        engine.register_fn("value", numeric_value);
        Self { engine }
    }

    pub fn new_scope() -> ScriptScope {
        Scope::new()
    }

    pub fn compile(&self, source: &str) -> Result<CompiledScript, CalcError> {
        self.engine
            .compile(source)
            .map(|ast| CompiledScript { ast })
            .map_err(script_error)
    }

    pub fn run(&self, source: &str) -> Result<String, CalcError> {
        let mut scope = Self::new_scope();
        self.run_with_scope(source, &mut scope)
    }

    pub fn run_with_scope(
        &self,
        source: &str,
        scope: &mut ScriptScope,
    ) -> Result<String, CalcError> {
        self.engine
            .eval_with_scope::<Dynamic>(scope, source)
            .map(dynamic_to_string)
            .map_err(script_error)
    }

    pub fn run_compiled(&self, script: &CompiledScript) -> Result<String, CalcError> {
        let mut scope = Self::new_scope();
        self.run_compiled_with_scope(script, &mut scope)
    }

    pub fn run_compiled_with_scope(
        &self,
        script: &CompiledScript,
        scope: &mut ScriptScope,
    ) -> Result<String, CalcError> {
        self.engine
            .eval_ast_with_scope::<Dynamic>(scope, &script.ast)
            .map(dynamic_to_string)
            .map_err(script_error)
    }
}

impl Default for ScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

fn calc_text(source: &str) -> String {
    evaluate_calc_statement(source, true)
}

fn simplify_text(source: &str) -> String {
    evaluate_calc_statement(source, false)
}

fn numeric_value(source: &str) -> f64 {
    let Ok(Statement::Eval(expr)) = parse_statement(source) else {
        return f64::NAN;
    };
    let env = Env::new();
    let simplified = simplify_with_env(expr, &env);
    eval_env(&simplified, &env).unwrap_or(f64::NAN)
}

fn evaluate_calc_statement(source: &str, prefer_numeric: bool) -> String {
    let mut env = Env::new();
    match parse_statement(source) {
        Ok(Statement::Assign(name, expr)) => {
            let simplified = simplify_with_env(expr, &env);
            match eval_env(&simplified, &env) {
                Ok(value) if prefer_numeric => {
                    env.set_var(&name, Expr::Float(value));
                    format!("{} = {}", name, format_number(value))
                }
                _ => {
                    env.set_var(&name, simplified.clone());
                    format!("{name} = {simplified}")
                }
            }
        }
        Ok(Statement::DefFn(name, params, body)) => {
            let len = params.len();
            env.set_fn(&name, UserFn { params, body });
            format!(
                "defined {}({} param{})",
                name,
                len,
                if len == 1 { "" } else { "s" }
            )
        }
        Ok(Statement::Eval(expr)) => {
            let simplified = simplify_with_env(expr, &env);
            if prefer_numeric {
                if let Ok(value) = eval_env(&simplified, &env) {
                    return format_number(value);
                }
            }
            simplified.to_string()
        }
        Err(err) => format!("error: {err}"),
    }
}

fn dynamic_to_string(value: Dynamic) -> String {
    if value.is_unit() {
        String::new()
    } else {
        value.to_string()
    }
}

fn script_error(error: impl core::fmt::Display) -> CalcError {
    CalcError::InvalidArgument(format!("script error: {error}"))
}

fn format_number(value: f64) -> String {
    if libm::fabs(value - libm::floor(value)) < 1e-12 && libm::fabs(value) < 9_007_199_254_740_992.0
    {
        format!("{value:.0}")
    } else {
        format!("{value:.12}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_should_call_calculator_parser_pipeline() {
        let runtime = ScriptRuntime::new();
        let result = runtime.run(r#"calc("2^10") == "1024""#).unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn compiled_script_should_run_with_persistent_scope() {
        let runtime = ScriptRuntime::new();
        let script = runtime
            .compile(r#"counter += 1; calc("sqrt(144)") + ":" + counter"#)
            .unwrap();
        let mut scope = ScriptRuntime::new_scope();
        scope.push("counter", 0_i64);

        assert_eq!(
            runtime
                .run_compiled_with_scope(&script, &mut scope)
                .unwrap(),
            "12:1"
        );
        assert_eq!(
            runtime
                .run_compiled_with_scope(&script, &mut scope)
                .unwrap(),
            "12:2"
        );
    }

    #[test]
    fn script_should_report_calculator_parse_errors_as_text() {
        let runtime = ScriptRuntime::new();
        let result = runtime.run(r#"calc("1 +")"#).unwrap();
        assert!(result.starts_with("error:"));
    }

    #[test]
    fn numeric_value_should_return_nan_when_expression_is_not_numeric() {
        let runtime = ScriptRuntime::new();
        let result = runtime.run(r#"value("x + 1").is_nan"#).unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn script_should_support_rhai_functions_and_loops() {
        let runtime = ScriptRuntime::new();
        let result = runtime
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
            .unwrap();
        assert_eq!(result, "30.0");
    }

    #[test]
    fn compile_should_return_error_for_invalid_rhai() {
        let runtime = ScriptRuntime::new();
        let Err(error) = runtime.compile("let = nope") else {
            panic!("invalid Rhai source compiled successfully");
        };
        assert!(error.to_string().contains("script error:"));
    }

    #[test]
    fn run_with_scope_should_keep_rhai_variables_between_runs() {
        let runtime = ScriptRuntime::new();
        let mut scope = ScriptRuntime::new_scope();

        assert_eq!(
            runtime
                .run_with_scope("let total = value(\"10 + 5\"); total", &mut scope)
                .unwrap(),
            "15.0"
        );
        assert_eq!(
            runtime
                .run_with_scope("total += value(\"sqrt(25)\"); total", &mut scope)
                .unwrap(),
            "20.0"
        );
    }

    #[test]
    fn script_should_compose_calculator_results_as_text() {
        let runtime = ScriptRuntime::new();
        let result = runtime
            .run(r#"calc("diff(x^3, x)") + " | " + simplify("sin(x)^2 + cos(x)^2")"#)
            .unwrap();
        assert_eq!(result, "3·x^2 | 1");
    }
}
