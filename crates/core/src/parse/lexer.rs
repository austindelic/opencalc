use crate::error::CalcError;
use crate::syntax;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    Bang,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Eq,
    EqEq,
    Eof,
}

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { src, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.src[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.advance();
        }
    }

    fn read_number(&mut self) -> Result<Token, CalcError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.advance();
        }
        if self.peek() == Some('.') {
            self.advance();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.advance();
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.advance();
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.advance();
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.advance();
            }
        }
        let s = &self.src[start..self.pos];
        // core::str::parse is available in no_std
        s.parse::<f64>()
            .map(Token::Number)
            .map_err(|_| CalcError::LexError(format!("invalid number: {}", s)))
    }

    fn read_ident(&mut self) -> Token {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            self.advance();
        }
        Token::Ident(self.src[start..self.pos].to_string())
    }

    pub fn next_token(&mut self) -> Result<Token, CalcError> {
        self.skip_whitespace();
        match self.peek() {
            None => Ok(Token::Eof),
            Some(c) => match c {
                '0'..='9' | '.' => self.read_number(),
                'a'..='z' | 'A'..='Z' | '_' => Ok(self.read_ident()),
                c if c == syntax::OP_ADD => {
                    self.advance();
                    Ok(Token::Plus)
                }
                c if c == syntax::OP_SUB => {
                    self.advance();
                    Ok(Token::Minus)
                }
                c if c == syntax::OP_MUL || syntax::OP_MUL_ALT.contains(&c) => {
                    self.advance();
                    Ok(Token::Star)
                }
                c if c == syntax::OP_DIV || c == syntax::OP_DIV_ALT => {
                    self.advance();
                    Ok(Token::Slash)
                }
                c if c == syntax::OP_POW => {
                    self.advance();
                    Ok(Token::Caret)
                }
                c if c == syntax::OP_MOD => {
                    self.advance();
                    Ok(Token::Percent)
                }
                c if c == syntax::OP_FACTORIAL => {
                    self.advance();
                    Ok(Token::Bang)
                }
                c if c == syntax::OP_LPAREN => {
                    self.advance();
                    Ok(Token::LParen)
                }
                c if c == syntax::OP_RPAREN => {
                    self.advance();
                    Ok(Token::RParen)
                }
                c if c == syntax::OP_LBRACKET => {
                    self.advance();
                    Ok(Token::LBracket)
                }
                c if c == syntax::OP_RBRACKET => {
                    self.advance();
                    Ok(Token::RBracket)
                }
                c if c == syntax::OP_COMMA => {
                    self.advance();
                    Ok(Token::Comma)
                }
                c if c == syntax::OP_SEMICOLON => {
                    self.advance();
                    Ok(Token::Semicolon)
                }
                c if c == syntax::OP_ASSIGN => {
                    self.advance();
                    if self.peek() == Some(syntax::OP_ASSIGN) {
                        self.advance();
                        Ok(Token::EqEq)
                    } else {
                        Ok(Token::Eq)
                    }
                }
                c => Err(CalcError::LexError(format!(
                    "unexpected character: '{}'",
                    c
                ))),
            },
        }
    }

    pub fn tokenize(src: &'a str) -> Result<Vec<Token>, CalcError> {
        let mut lexer = Lexer::new(src);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token()?;
            let done = tok == Token::Eof;
            tokens.push(tok);
            if done {
                break;
            }
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tokens() {
        let toks = Lexer::tokenize("1 + 2.5 * x").unwrap();
        assert_eq!(
            toks,
            vec![
                Token::Number(1.0),
                Token::Plus,
                Token::Number(2.5),
                Token::Star,
                Token::Ident("x".into()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn scientific_notation() {
        let toks = Lexer::tokenize("1e3").unwrap();
        assert_eq!(toks[0], Token::Number(1000.0));
    }
}
