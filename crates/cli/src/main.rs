use core::{
    eval_env, parse_statement, simplify_with_env, Env, Expr, ScriptRuntime, ScriptScope, Statement,
    UserFn,
};
use std::io::{self, BufRead, Write};

fn main() {
    println!("opencalc  —  type an expression, '! <rhai>', ':script', ':run file.rhai', or :help");

    let mut env = Env::new();
    let scripts = ScriptRuntime::new();
    let mut script_scope = ScriptRuntime::new_scope();
    let mut script_mode = false;
    let stdin = io::stdin();

    loop {
        print!("{}", if script_mode { "rhai> " } else { "> " });
        io::stdout().flush().unwrap();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        if script_mode {
            match input {
                ":calc" | ":!" | ":script" => {
                    script_mode = false;
                    println!("calculator mode");
                }
                ":q" | "quit" | "exit" => break,
                ":help" => print_script_help(),
                _ => run_script_line(&scripts, &mut script_scope, input),
            }
            continue;
        }

        // ── REPL commands ─────────────────────────────────────────────────────
        match input {
            ":q" | "quit" | "exit" => break,
            ":help" => {
                print_help();
                continue;
            }
            ":vars" => {
                if env.vars.is_empty() {
                    println!("(no variables)");
                } else {
                    for (name, val) in &env.vars {
                        println!("  {} = {}", name, val);
                    }
                }
                continue;
            }
            ":fns" => {
                if env.fns.is_empty() {
                    println!("(no user functions)");
                } else {
                    for (name, f) in &env.fns {
                        println!("  {}({}) = {}", name, f.params.join(", "), f.body);
                    }
                }
                continue;
            }
            ":clear" => {
                env = Env::new();
                println!("environment cleared");
                continue;
            }
            ":script" | "!" => {
                script_mode = true;
                println!("Rhai mode. Type :calc to return to calculator mode.");
                continue;
            }
            _ => {}
        }

        if let Some(script) = input.strip_prefix('!') {
            run_script_line(&scripts, &mut script_scope, script.trim());
            continue;
        }

        if let Some(path) = input.strip_prefix(":run ") {
            run_script_file(&scripts, &mut script_scope, path.trim());
            continue;
        }

        // ── Normal expression / assignment / def ──────────────────────────────
        match parse_statement(input) {
            Ok(Statement::Assign(name, expr)) => {
                let simplified = simplify_with_env(expr, &env);
                match eval_env(&simplified, &env) {
                    Ok(v) => {
                        env.set_var(&name, Expr::Float(v));
                        println!("{} = {}", name, v);
                    }
                    Err(_) => {
                        env.set_var(&name, simplified.clone());
                        println!("{} = {}", name, simplified);
                    }
                }
            }
            Ok(Statement::DefFn(name, params, body)) => {
                let n = params.len();
                env.set_fn(&name, UserFn { params, body });
                println!(
                    "defined {}({} param{})",
                    name,
                    n,
                    if n == 1 { "" } else { "s" }
                );
            }
            Ok(Statement::Eval(expr)) => {
                let simplified = simplify_with_env(expr, &env);
                match eval_env(&simplified, &env) {
                    Ok(v) => println!("{}", v),
                    Err(_) => println!("{}", simplified),
                }
            }
            Err(e) => eprintln!("error: {}", e),
        }
    }
}

fn run_script_line(scripts: &ScriptRuntime, scope: &mut ScriptScope, input: &str) {
    if input.is_empty() {
        return;
    }
    match scripts.run_with_scope(input, scope) {
        Ok(output) if output.is_empty() => {}
        Ok(output) => println!("{output}"),
        Err(err) => eprintln!("{err}"),
    }
}

fn run_script_file(scripts: &ScriptRuntime, scope: &mut ScriptScope, path: &str) {
    match std::fs::read_to_string(path) {
        Ok(source) => match scripts.compile(&source) {
            Ok(script) => match scripts.run_compiled_with_scope(&script, scope) {
                Ok(output) if output.is_empty() => {}
                Ok(output) => println!("{output}"),
                Err(err) => eprintln!("{err}"),
            },
            Err(err) => eprintln!("{err}"),
        },
        Err(err) => eprintln!("error: cannot read {path}: {err}"),
    }
}

fn print_help() {
    println!(
        r#"
opencalc — symbolic CAS calculator

Expressions:
  2 + 3 * sin(pi/6)        arithmetic with all standard functions
  x = 5                    assign variable
  fn f(x) =x^2 + 1       define function
  f(3)                     call user function

Symbolic ops:
  diff(x^3, x)             derivative
  diff(x^3, x, 2)          nth derivative
  integrate(x^2, x)        indefinite integral
  solve(x^2 - 4, x)        roots of equation
  solve(x^2 == 4, x)       solve equation
  taylor(sin(x), x, 0, 5)  Taylor series around 0, order 5
  expand((x+1)^3)          polynomial expansion

Matrix / list:
  [[1,2],[3,4]]             matrix literal
  zeros(3)  ones(2,3)  eye(4)
  det(A)  tr(A)  transpose(A)  inv(A)
  dot([1,2],[3,4])  cross([1,0,0],[0,1,0])
  range(5)  range(1,10)

REPL commands:
  :vars   show variables
  :fns    show user functions
  :clear  reset environment
  ! expr  run one Rhai script line
  :script enter persistent Rhai REPL mode
  :run path.rhai compile and run a Rhai script file
  :help   this message
  :q      quit

Rhai scripting:
  ! calc("diff(x^3, x)")       run opencalc parser from Rhai
  ! simplify("(x+1)(x-1)")     symbolic result as text
  ! value("2^10") + 1          numeric result as f64
"#
    );
}

fn print_script_help() {
    println!(
        r#"
Rhai mode

Examples:
  calc("2^10")
  simplify("diff(x^3, x)")
  let x = value("sqrt(144)"); x + 1

Commands:
  :calc   return to calculator mode
  :help   this message
  :q      quit
"#
    );
}
