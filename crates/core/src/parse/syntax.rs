/// Single source of truth for all syntax — keywords, named constants,
/// and operator characters.  Edit here to change the language surface.

// ── Keywords ──────────────────────────────────────────────────────────────────

/// Keyword that introduces a user-defined function: `fn f(x) = body`
pub const KW_DEF: &str = "fn";

// ── Named constants ───────────────────────────────────────────────────────────

/// All accepted spellings of π.
pub const NAMES_PI: &[&str] = &["pi", "PI"];
/// All accepted spellings of Euler's number.
pub const NAMES_E: &[&str] = &["e"];
/// All accepted spellings of the imaginary unit.
pub const NAMES_I: &[&str] = &["i", "I"];
/// All accepted spellings of +∞.
pub const NAMES_INF: &[&str] = &["inf", "Inf", "infinity"];

// ── Operator characters ───────────────────────────────────────────────────────

pub const OP_ADD: char = '+';
pub const OP_SUB: char = '-';
/// Primary multiplication character.
pub const OP_MUL: char = '*';
/// Unicode synonyms for multiplication (×, middle-dot).
pub const OP_MUL_ALT: &[char] = &['\u{00D7}', '\u{00B7}']; // × ·
/// Primary division character.
pub const OP_DIV: char = '/';
/// Unicode synonym for division (÷).
pub const OP_DIV_ALT: char = '\u{00F7}'; // ÷
pub const OP_POW: char = '^';
pub const OP_MOD: char = '%';
pub const OP_FACTORIAL: char = '!';
/// Single `=` used for assignment and the first half of `==`.
pub const OP_ASSIGN: char = '=';
pub const OP_LPAREN: char = '(';
pub const OP_RPAREN: char = ')';
pub const OP_LBRACKET: char = '[';
pub const OP_RBRACKET: char = ']';
pub const OP_COMMA: char = ',';
pub const OP_SEMICOLON: char = ';';
