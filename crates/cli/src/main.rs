use std::io::{self, BufRead, Write};
use core::{parse_statement, simplify, eval_env, Statement, Env, Expr, UserFn};

fn main() {
    println!("opencalc  —  type an expression, 'fn f(x) =body', 'x = expr', or :help");

    let mut env = Env::new();
    let stdin = io::stdin();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let input = line.trim();
        if input.is_empty() { continue; }

        // ── REPL commands ─────────────────────────────────────────────────────
        match input {
            ":q" | "quit" | "exit" => break,
            ":help" => { print_help(); continue; }
            ":vars" => {
                if env.vars.is_empty() {
                    println!("(no variables)");
                } else {
                    for (name, val) in &env.vars { println!("  {} = {}", name, val); }
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
            ":clear" => { env = Env::new(); println!("environment cleared"); continue; }
            _ => {}
        }

        // ── Normal expression / assignment / def ──────────────────────────────
        match parse_statement(input) {
            Ok(Statement::Assign(name, expr)) => {
                let simplified = simplify(expr);
                match eval_env(&simplified, &env) {
                    Ok(v) => { env.set_var(&name, Expr::Float(v)); println!("{} = {}", name, v); }
                    Err(_) => { env.set_var(&name, simplified.clone()); println!("{} = {}", name, simplified); }
                }
            }
            Ok(Statement::DefFn(name, params, body)) => {
                let n = params.len();
                env.set_fn(&name, UserFn { params, body });
                println!("defined {}({} param{})", name, n, if n == 1 { "" } else { "s" });
            }
            Ok(Statement::Eval(expr)) => {
                let simplified = simplify(expr);
                match eval_env(&simplified, &env) {
                    Ok(v)  => println!("{}", v),
                    Err(_) => println!("{}", simplified),
                }
            }
            Err(e) => eprintln!("error: {}", e),
        }
    }
}

fn print_help() {
    println!(r#"
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
  :help   this message
  :q      quit
"#);
}
