use alloc::string::String;

#[derive(Debug, Clone, PartialEq)]
pub enum CalcError {
    LexError(String),
    ParseError(String),
    DivisionByZero,
    UndefinedVariable(String),
    InvalidArgument(String),
    Overflow,
}

impl core::fmt::Display for CalcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CalcError::LexError(s) => write!(f, "lex error: {}", s),
            CalcError::ParseError(s) => write!(f, "parse error: {}", s),
            CalcError::DivisionByZero => write!(f, "division by zero"),
            CalcError::UndefinedVariable(s) => write!(f, "undefined variable: {}", s),
            CalcError::InvalidArgument(s) => write!(f, "invalid argument: {}", s),
            CalcError::Overflow => write!(f, "arithmetic overflow"),
        }
    }
}
