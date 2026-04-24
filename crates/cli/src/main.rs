use std::io::{self, BufRead, Write};
use opencalc_core::{parse, simplify, eval, Context};

fn main() {
    println!("opencalc CLI — type an expression or :q to quit");

    let stdin = io::stdin();
    let mut ctx = Context::new();

    for line in stdin.lock().lines() {
        let line = line.expect("failed to read line");
        let input = line.trim();

        if input.is_empty() { continue; }
        if input == ":q" || input == "quit" || input == "exit" { break; }

        // Variable assignment:  x = <expr>
        if let Some((name, rhs)) = input.split_once('=') {
            let name = name.trim();
            let rhs  = rhs.trim();
            if name.chars().all(|c| c.is_alphanumeric() || c == '_') && !name.is_empty() {
                match parse(rhs).map(simplify) {
                    Ok(expr) => match eval(&expr, &ctx) {
                        Ok(v) => {
                            ctx.set(name, v);
                            println!("{} = {}", name, v);
                        }
                        Err(e) => eprintln!("eval error: {}", e),
                    },
                    Err(e) => eprintln!("parse error: {}", e),
                }
                print!("> ");
                io::stdout().flush().unwrap();
                continue;
            }
        }

        match parse(input).map(simplify) {
            Ok(expr) => {
                // Try numeric eval first; fall back to showing the simplified form
                match eval(&expr, &ctx) {
                    Ok(v)  => println!("{}", v),
                    Err(_) => println!("{}", expr),
                }
            }
            Err(e) => eprintln!("error: {}", e),
        }

        print!("> ");
        io::stdout().flush().unwrap();
    }
}
