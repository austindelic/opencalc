use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use crate::error::CalcError;
use crate::expr::Expr;

fn check_nonempty(m: &[Vec<Expr>], label: &str) -> Result<(usize, usize), CalcError> {
    if m.is_empty() || m[0].is_empty() {
        Err(CalcError::InvalidArgument(
            alloc::format!("{}: matrix must be non-empty", label)
        ))
    } else {
        Ok((m.len(), m[0].len()))
    }
}

/// Add two matrices element-wise.
pub fn mat_add(a: &[Vec<Expr>], b: &[Vec<Expr>]) -> Result<Vec<Vec<Expr>>, CalcError> {
    let (rows_a, cols_a) = check_nonempty(a, "mat_add")?;
    let (rows_b, cols_b) = check_nonempty(b, "mat_add")?;
    if rows_a != rows_b || cols_a != cols_b {
        return Err(CalcError::InvalidArgument("mat_add: dimension mismatch".into()));
    }
    Ok(a.iter().zip(b.iter()).map(|(ra, rb)| {
        ra.iter().zip(rb.iter())
            .map(|(ea, eb)| Expr::Add(vec![ea.clone(), eb.clone()]))
            .collect()
    }).collect())
}

/// Multiply two matrices symbolically.
pub fn mat_mul(a: &[Vec<Expr>], b: &[Vec<Expr>]) -> Result<Vec<Vec<Expr>>, CalcError> {
    let (rows_a, cols_a) = check_nonempty(a, "mat_mul")?;
    let (_, cols_b) = check_nonempty(b, "mat_mul")?;
    if cols_a != b.len() {
        return Err(CalcError::InvalidArgument("mat_mul: inner dimensions must match".into()));
    }
    let mut result = Vec::with_capacity(rows_a);
    for i in 0..rows_a {
        let mut row = Vec::with_capacity(cols_b);
        for j in 0..cols_b {
            let terms: Vec<Expr> = (0..cols_a)
                .map(|k| Expr::Mul(vec![a[i][k].clone(), b[k][j].clone()]))
                .collect();
            row.push(Expr::Add(terms));
        }
        result.push(row);
    }
    Ok(result)
}

/// Transpose a matrix.
pub fn mat_transpose(m: &[Vec<Expr>]) -> Result<Vec<Vec<Expr>>, CalcError> {
    let (rows, cols) = check_nonempty(m, "mat_transpose")?;
    Ok((0..cols).map(|j| (0..rows).map(|i| m[i][j].clone()).collect()).collect())
}

/// Sum of diagonal elements.
pub fn mat_trace(m: &[Vec<Expr>]) -> Result<Expr, CalcError> {
    let (rows, cols) = check_nonempty(m, "mat_trace")?;
    let n = rows.min(cols);
    let diag: Vec<Expr> = (0..n).map(|i| m[i][i].clone()).collect();
    Ok(Expr::Add(diag))
}

/// Determinant via cofactor expansion (recursive).
pub fn mat_det(m: &[Vec<Expr>]) -> Result<Expr, CalcError> {
    let (rows, cols) = check_nonempty(m, "mat_det")?;
    if rows != cols {
        return Err(CalcError::InvalidArgument("mat_det: requires square matrix".into()));
    }
    if m.iter().any(|r| r.len() != rows) {
        return Err(CalcError::InvalidArgument("mat_det: jagged matrix".into()));
    }
    Ok(det_recursive(m))
}

fn det_recursive(m: &[Vec<Expr>]) -> Expr {
    let n = m.len();
    if n == 1 {
        return m[0][0].clone();
    }
    if n == 2 {
        return Expr::Add(vec![
            Expr::Mul(vec![m[0][0].clone(), m[1][1].clone()]),
            Expr::Neg(Box::new(Expr::Mul(vec![m[0][1].clone(), m[1][0].clone()]))),
        ]);
    }
    let terms: Vec<Expr> = (0..n).map(|col| {
        let cofactor = det_recursive(&submatrix(m, 0, col));
        let term = Expr::Mul(vec![m[0][col].clone(), cofactor]);
        if col % 2 == 0 { term } else { Expr::Neg(Box::new(term)) }
    }).collect();
    Expr::Add(terms)
}

fn submatrix(m: &[Vec<Expr>], skip_row: usize, skip_col: usize) -> Vec<Vec<Expr>> {
    m.iter().enumerate()
        .filter(|(i, _)| *i != skip_row)
        .map(|(_, row)| {
            row.iter().enumerate()
                .filter(|(j, _)| *j != skip_col)
                .map(|(_, e)| e.clone())
                .collect()
        })
        .collect()
}

/// Matrix inverse via symbolic Gaussian elimination.
pub fn mat_inv(m: &[Vec<Expr>]) -> Result<Vec<Vec<Expr>>, CalcError> {
    let (rows, cols) = check_nonempty(m, "mat_inv")?;
    if rows != cols {
        return Err(CalcError::InvalidArgument("mat_inv: requires square matrix".into()));
    }
    if m.iter().any(|r| r.len() != rows) {
        return Err(CalcError::InvalidArgument("mat_inv: jagged matrix".into()));
    }
    let n = rows;

    // Build augmented [M | I]
    let mut aug: Vec<Vec<Expr>> = m.iter().enumerate().map(|(i, row)| {
        let mut r = row.clone();
        for j in 0..n {
            r.push(if i == j { Expr::one() } else { Expr::zero() });
        }
        r
    }).collect();

    for col in 0..n {
        let pivot_row = (col..n)
            .find(|&r| !aug[r][col].is_zero())
            .ok_or_else(|| CalcError::InvalidArgument("mat_inv: matrix is singular".into()))?;
        aug.swap(col, pivot_row);

        let pivot = aug[col][col].clone();
        let inv_pivot = Expr::Pow(Box::new(pivot), Box::new(Expr::neg_one()));

        for j in 0..(2 * n) {
            let val = aug[col][j].clone();
            aug[col][j] = Expr::Mul(vec![inv_pivot.clone(), val]);
        }

        for row in 0..n {
            if row == col { continue; }
            let factor = aug[row][col].clone();
            if factor.is_zero() { continue; }
            for j in 0..(2 * n) {
                let sub = Expr::Mul(vec![factor.clone(), aug[col][j].clone()]);
                let cur = aug[row][j].clone();
                aug[row][j] = Expr::Add(vec![cur, Expr::Neg(Box::new(sub))]);
            }
        }
    }

    Ok(aug.into_iter().map(|row| row[n..].to_vec()).collect())
}

/// 3D cross product.
pub fn cross3(a: &[Expr], b: &[Expr]) -> Result<Vec<Expr>, CalcError> {
    if a.len() != 3 || b.len() != 3 {
        return Err(CalcError::InvalidArgument("cross3: requires two 3D vectors".into()));
    }
    Ok(vec![
        Expr::Add(vec![
            Expr::Mul(vec![a[1].clone(), b[2].clone()]),
            Expr::Neg(Box::new(Expr::Mul(vec![a[2].clone(), b[1].clone()]))),
        ]),
        Expr::Add(vec![
            Expr::Mul(vec![a[2].clone(), b[0].clone()]),
            Expr::Neg(Box::new(Expr::Mul(vec![a[0].clone(), b[2].clone()]))),
        ]),
        Expr::Add(vec![
            Expr::Mul(vec![a[0].clone(), b[1].clone()]),
            Expr::Neg(Box::new(Expr::Mul(vec![a[1].clone(), b[0].clone()]))),
        ]),
    ])
}

/// Dot product of two equal-length vectors.
pub fn dot(a: &[Expr], b: &[Expr]) -> Result<Expr, CalcError> {
    if a.is_empty() || a.len() != b.len() {
        return Err(CalcError::InvalidArgument("dot: vectors must be non-empty and equal length".into()));
    }
    let terms: Vec<Expr> = a.iter().zip(b.iter())
        .map(|(ai, bi)| Expr::Mul(vec![ai.clone(), bi.clone()]))
        .collect();
    Ok(Expr::Add(terms))
}
