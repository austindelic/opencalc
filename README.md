# opencalc

A symbolic CAS (computer algebra system) calculator written in Rust. Runs as a native desktop GUI or a terminal REPL. The core engine is `#![no_std]` and embeds on bare-metal targets.

---

## Features

### Arithmetic
- Integer, decimal, and rational exact arithmetic (`1/3 + 1/6 = 1/2`)
- Standard operators: `+` `-` `*` `/` `^` `%` `!`
- Unicode operators accepted: `×` `÷` `·`
- Operator precedence and right-associative exponentiation (`2^3^4`)

### Constants
| Input | Value |
|---|---|
| `pi`, `PI` | π |
| `e` | Euler's number |
| `i`, `I` | imaginary unit |
| `inf`, `Inf` | ∞ |

### Variables & Functions
```
x = 5
y = x^2 + 1
fn f(x) = x^2 + 2*x + 1
def g(x, y) = sqrt(x^2 + y^2)
f(3)
```

### Symbolic Calculus
| Expression | Result |
|---|---|
| `diff(x^3, x)` | `3·x^2` |
| `diff(sin(x), x, 2)` | `-sin(x)` (2nd derivative) |
| `integrate(x^2, x)` | `1/3·x^3` |
| `solve(x^2 - 4, x)` | `[2, -2]` |
| `solve(x^2 == 4, x)` | `[2, -2]` |
| `taylor(sin(x), x, 0, 5)` | Taylor series to order 5 |
| `expand((x+1)^3)` | `x^3 + 3·x^2 + 3·x + 1` |

`integrate` handles: constants, power rule, `sin`, `cos`, `exp`, `ln`, `sqrt`, and scalar multiples / sums of the above. Falls back to `nintegrate` for numerical definite integrals.

`solve` uses analytic roots for linear and quadratic, Newton-Raphson for higher degree.

### Built-in Functions

**Trigonometry**
`sin` `cos` `tan` `asin` `acos` `atan` `atan2`
`sinh` `cosh` `tanh` `asinh` `acosh` `atanh`

**Exponential / Logarithm**
`exp` `ln` `log` `log2` `log10`

**Roots**
`sqrt` `cbrt`

**Rounding / Sign**
`abs` `floor` `ceil` `round` `sign`

**Number Theory**
`factorial` (or `n!`) `gcd` `lcm` `mod` `isprime`

**Extrema**
`max` `min`

**Complex**
`re` `im` `conj` `arg`

**Numerical Calculus**
`ndiff(expr, var, point)` — numerical derivative at a point
`nintegrate(expr, var, a, b)` — numerical definite integral (Simpson's rule)

**Sequences / Summation**
`sum(expr, var, from, to)` — symbolic summation
`product(expr, var, from, to)`
`range(n)` → `[0, 1, …, n-1]`
`range(start, end)`
`range(start, end, step)`

**Logic**
`if(cond, then, else)`

**Misc**
`random()` `random(lo, hi)`
`numer(x)` `denom(x)`
`simplify(expr)` `expand(expr)`

### Matrices & Vectors
```
A = [[1, 2], [3, 4]]
det(A)            → -2
tr(A)             → 5
transpose(A)
inv(A)
zeros(3)          → 3×3 zero matrix
zeros(2, 3)       → 2×3 zero matrix
ones(2)
eye(4)            → 4×4 identity matrix
dot([1,2,3], [4,5,6])
cross([1,0,0], [0,1,0])
norm([3, 4])      → 5
len([1,2,3])      → 3
```

---

## Targets

| Crate | Description |
|---|---|
| `core` | `#![no_std]` CAS engine — shared by all targets |
| `gui` | Native desktop app (egui / eframe) |
| `cli` | Terminal REPL |
| `embedded` | `#![no_std] #![no_main]` bare-metal target |

---

## Building

```sh
# GUI (default)
cargo run

# CLI REPL
cargo run -p cli

# Release builds
cargo build -p gui --release
cargo build -p cli --profile release-cli

# Tests
cargo test -p core --lib
```

---

## CLI Commands

```
:vars    show all variables
:fns     show all user-defined functions
:clear   reset the environment
:help    show help
:q       quit
```
