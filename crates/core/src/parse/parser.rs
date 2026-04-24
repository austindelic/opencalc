use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::error::CalcError;
use crate::expr::{BuiltinFn, Constant, Expr};
use crate::lexer::{Lexer, Token};
use crate::rational::Rational;

// ── Statement ─────────────────────────────────────────────────────────────────

/// A top-level input statement (CLI / GUI).
pub enum Statement {
    /// `name = expr`
    Assign(String, Expr),
    /// `def name(params…) = body`
    DefFn(String, Vec<String>, Expr),
    /// Bare expression to evaluate / display
    Eval(Expr),
}

/// Parse a top-level statement.
pub fn parse_statement(src: &str) -> Result<Statement, CalcError> {
    let src = src.trim();

    // "def name(params) = body"
    let kw = crate::syntax::KW_DEF;
    if src.starts_with(kw) && src[kw.len()..].starts_with(' ') {
        return parse_def(src[kw.len() + 1..].trim());
    }

    // Simple variable assignment: <ident> = <expr>   (not ==)
    let tokens = Lexer::tokenize(src)?;
    if tokens.len() >= 3 {
        if let (Token::Ident(name), Token::Eq) = (&tokens[0], &tokens[1]) {
            // Slice from index 2 to end (includes Eof)
            let name = name.clone();
            let mut p = Parser::new(tokens[2..].to_vec());
            let expr = p.parse_equation()?;
            p.expect(Token::Eof)?;
            return Ok(Statement::Assign(name, expr));
        }
    }

    // Bare expression
    let mut p = Parser::new(tokens);
    let expr = p.parse_equation()?;
    p.expect(Token::Eof)?;
    Ok(Statement::Eval(expr))
}

fn parse_def(src: &str) -> Result<Statement, CalcError> {
    let lparen = src.find('(')
        .ok_or_else(|| CalcError::ParseError("def: expected '('".into()))?;
    let name = src[..lparen].trim().to_string();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(CalcError::ParseError("def: invalid function name".into()));
    }
    let rest = &src[lparen + 1..];
    let rparen = rest.find(')')
        .ok_or_else(|| CalcError::ParseError("def: expected ')'".into()))?;
    let params: Vec<String> = {
        let ps = rest[..rparen].trim();
        if ps.is_empty() { vec![] }
        else { ps.split(',').map(|p| p.trim().to_string()).collect() }
    };
    let body_src = rest[rparen + 1..].trim()
        .strip_prefix('=')
        .ok_or_else(|| CalcError::ParseError("def: expected '=' after params".into()))?
        .trim();
    let body = parse(body_src)?;
    Ok(Statement::DefFn(name, params, body))
}

// ── Expression parser ─────────────────────────────────────────────────────────

/// Parse a mathematical expression string into an `Expr` tree.
pub fn parse(src: &str) -> Result<Expr, CalcError> {
    let tokens = Lexer::tokenize(src)?;
    let mut p = Parser::new(tokens);
    let expr = p.parse_equation()?;
    p.expect(Token::Eof)?;
    Ok(expr)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self { Parser { tokens, pos: 0 } }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len() { self.pos += 1; }
        tok
    }

    fn expect(&mut self, expected: Token) -> Result<(), CalcError> {
        let got = self.advance();
        if got == expected { Ok(()) }
        else { Err(CalcError::ParseError(format!("expected {:?}, got {:?}", expected, got))) }
    }

    // equation = additive ('==' additive)?
    fn parse_equation(&mut self) -> Result<Expr, CalcError> {
        let lhs = self.parse_additive()?;
        if self.peek() == &Token::EqEq {
            self.advance();
            let rhs = self.parse_additive()?;
            Ok(Expr::Equation(Box::new(lhs), Box::new(rhs)))
        } else {
            Ok(lhs)
        }
    }

    // additive = term (('+' | '-') term)*
    fn parse_additive(&mut self) -> Result<Expr, CalcError> {
        let mut lhs = self.parse_term()?;
        loop {
            match self.peek() {
                Token::Plus  => { self.advance(); let rhs = self.parse_term()?; lhs = flatten_add(lhs, rhs); }
                Token::Minus => { self.advance(); let rhs = self.parse_term()?; lhs = flatten_add(lhs, Expr::Neg(Box::new(rhs))); }
                _ => break,
            }
        }
        Ok(lhs)
    }

    // term = unary (('*' | '/' | '%' | <implicit>) unary)*
    // Implicit multiplication fires when the next token can start a factor:
    //   2x   3pi   2sin(x)   (x+1)(x+2)   2(x+1)
    fn parse_term(&mut self) -> Result<Expr, CalcError> {
        let mut lhs = self.parse_unary()?;
        loop {
            match self.peek() {
                Token::Star    => { self.advance(); let rhs = self.parse_unary()?; lhs = flatten_mul(lhs, rhs); }
                Token::Slash   => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    let inv = Expr::Pow(Box::new(rhs), Box::new(Expr::neg_one()));
                    lhs = flatten_mul(lhs, inv);
                }
                Token::Percent => { self.advance(); let rhs = self.parse_unary()?; lhs = Expr::Call(BuiltinFn::Mod, vec![lhs, rhs]); }
                // implicit ×: number, identifier, or opening paren directly after a factor
                Token::Number(_) | Token::Ident(_) | Token::LParen => {
                    let rhs = self.parse_unary()?;
                    lhs = flatten_mul(lhs, rhs);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    // unary = ('-' | '+') unary | factorial
    fn parse_unary(&mut self) -> Result<Expr, CalcError> {
        match self.peek() {
            Token::Minus => { self.advance(); Ok(Expr::Neg(Box::new(self.parse_unary()?))) }
            Token::Plus  => { self.advance(); self.parse_unary() }
            _            => self.parse_factorial(),
        }
    }

    // factorial = power '!'*
    fn parse_factorial(&mut self) -> Result<Expr, CalcError> {
        let mut expr = self.parse_power()?;
        while self.peek() == &Token::Bang {
            self.advance();
            expr = Expr::Call(BuiltinFn::Factorial, vec![expr]);
        }
        Ok(expr)
    }

    // power = primary ('^' unary)?   right-associative
    fn parse_power(&mut self) -> Result<Expr, CalcError> {
        let base = self.parse_primary()?;
        if self.peek() == &Token::Caret {
            self.advance();
            let exp = self.parse_unary()?;
            Ok(Expr::Pow(Box::new(base), Box::new(exp)))
        } else {
            Ok(base)
        }
    }

    // primary = number | ident [call] | '(' expr ')' | '[' list ']'
    fn parse_primary(&mut self) -> Result<Expr, CalcError> {
        match self.peek().clone() {
            Token::Number(n) => {
                self.advance();
                let is_int = n.abs() < 9.007e15 && n == (n as i64) as f64;
                if is_int { Ok(Expr::Rat(Rational::from(n as i64))) }
                else      { Ok(Expr::Float(n)) }
            }
            Token::Ident(name) => {
                self.advance();
                if self.peek() == &Token::LParen {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(Token::RParen)?;
                    if let Some(func) = BuiltinFn::from_name(&name) {
                        Ok(Expr::Call(func, args))
                    } else {
                        // Unknown ident → user-defined function call
                        Ok(Expr::FnCall(name, args))
                    }
                } else {
                    {
                        use crate::syntax;
                        let n = name.as_str();
                        Ok(if syntax::NAMES_PI.contains(&n) {
                            Expr::Const(Constant::Pi)
                        } else if syntax::NAMES_E.contains(&n) {
                            Expr::Const(Constant::E)
                        } else if syntax::NAMES_I.contains(&n) {
                            Expr::Const(Constant::I)
                        } else if syntax::NAMES_INF.contains(&n) {
                            Expr::Const(Constant::Inf)
                        } else {
                            Expr::Var(name)
                        })
                    }
                }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_equation()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => {
                self.advance();
                if self.peek() == &Token::RBracket {
                    self.advance();
                    return Ok(Expr::List(vec![]));
                }
                let mut items = vec![self.parse_equation()?];
                while self.peek() == &Token::Comma {
                    self.advance();
                    items.push(self.parse_equation()?);
                }
                self.expect(Token::RBracket)?;
                // If all items are lists → matrix
                if items.iter().all(|e| matches!(e, Expr::List(_))) {
                    let rows: Vec<Vec<Expr>> = items.into_iter()
                        .filter_map(|e| if let Expr::List(row) = e { Some(row) } else { None })
                        .collect();
                    Ok(Expr::Matrix(rows))
                } else {
                    Ok(Expr::List(items))
                }
            }
            tok => Err(CalcError::ParseError(format!("unexpected token: {:?}", tok))),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, CalcError> {
        if self.peek() == &Token::RParen { return Ok(vec![]); }
        let mut args = vec![self.parse_equation()?];
        while self.peek() == &Token::Comma {
            self.advance();
            args.push(self.parse_equation()?);
        }
        Ok(args)
    }
}

fn flatten_add(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Add(mut l), Expr::Add(r)) => { l.extend(r); Expr::Add(l) }
        (Expr::Add(mut l), r)            => { l.push(r);   Expr::Add(l) }
        (l, Expr::Add(mut r))            => { r.insert(0, l); Expr::Add(r) }
        (l, r)                           => Expr::Add(vec![l, r]),
    }
}

fn flatten_mul(lhs: Expr, rhs: Expr) -> Expr {
    match (lhs, rhs) {
        (Expr::Mul(mut l), Expr::Mul(r)) => { l.extend(r); Expr::Mul(l) }
        (Expr::Mul(mut l), r)            => { l.push(r);   Expr::Mul(l) }
        (l, Expr::Mul(mut r))            => { r.insert(0, l); Expr::Mul(r) }
        (l, r)                           => Expr::Mul(vec![l, r]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_arith() {
        let e = parse("1 + 2").unwrap();
        assert_eq!(e, Expr::Add(vec![Expr::integer(1), Expr::integer(2)]));
    }

    #[test]
    fn power_right_assoc() {
        let e = parse("2^3^4").unwrap();
        assert_eq!(e, Expr::Pow(
            Box::new(Expr::integer(2)),
            Box::new(Expr::Pow(Box::new(Expr::integer(3)), Box::new(Expr::integer(4)))),
        ));
    }

    #[test]
    fn negative_unary() {
        let e = parse("-x").unwrap();
        assert_eq!(e, Expr::Neg(Box::new(Expr::Var("x".into()))));
    }

    #[test]
    fn function_call() {
        let e = parse("sin(x)").unwrap();
        assert_eq!(e, Expr::Call(BuiltinFn::Sin, vec![Expr::Var("x".into())]));
    }

    #[test]
    fn user_fn_call() {
        let e = parse("f(x, y)").unwrap();
        assert_eq!(e, Expr::FnCall("f".into(), vec![Expr::Var("x".into()), Expr::Var("y".into())]));
    }

    #[test]
    fn equation() {
        let e = parse("x^2 == 4").unwrap();
        assert!(matches!(e, Expr::Equation(..)));
    }

    #[test]
    fn list_literal() {
        let e = parse("[1, 2, 3]").unwrap();
        assert!(matches!(e, Expr::List(_)));
    }

    #[test]
    fn matrix_literal() {
        let e = parse("[[1, 2], [3, 4]]").unwrap();
        assert!(matches!(e, Expr::Matrix(_)));
    }

    #[test]
    fn factorial() {
        let e = parse("5!").unwrap();
        assert_eq!(e, Expr::Call(BuiltinFn::Factorial, vec![Expr::integer(5)]));
    }

    #[test]
    fn implicit_mul_number_var() {
        // 2x → 2 * x
        let e = parse("2x").unwrap();
        assert_eq!(e, Expr::Mul(vec![Expr::integer(2), Expr::Var("x".into())]));
    }

    #[test]
    fn implicit_mul_parens() {
        // (x+1)(x+2) — both sides are Add, result is Mul of two Adds
        let e = parse("(x+1)(x+2)").unwrap();
        assert!(matches!(e, Expr::Mul(_)));
    }

    #[test]
    fn implicit_mul_number_paren() {
        // 2(x+1) → 2 * (x+1)
        let e = parse("2(x+1)").unwrap();
        assert!(matches!(e, Expr::Mul(_)));
    }

    #[test]
    fn division_as_mul_inv() {
        let e = parse("a/b").unwrap();
        assert_eq!(e, Expr::Mul(vec![
            Expr::Var("a".into()),
            Expr::Pow(Box::new(Expr::Var("b".into())), Box::new(Expr::neg_one())),
        ]));
    }
}
