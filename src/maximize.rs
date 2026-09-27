/// [`maximize`] failure.
///
/// Usually this means invalid input.
#[derive(Debug)]
pub struct MaximizeError;

impl core::error::Error for MaximizeError {}

impl core::fmt::Display for MaximizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("`lprog::maximize` failed")
    }
}

/// Maximize objective function while satisfying constraints.
///
/// This is a low-level function that accepts constraints only in standard form, but works without `alloc` feature.
/// For an easy-to-use alternative see [`optimize`](crate::optimize).
///
/// Objective function is defined as
/// _f(a<sub>0</sub>, …, a<sub>n</sub>) = a<sub>0</sub> + a<sub>1</sub>x<sub>1</sub> + … + a<sub>n</sub>x<sub>n</sub>_ where
/// - _a<sub>0</sub>, …, a<sub>n</sub>_ are known coefficients of the objective function,
/// - _x<sub>1</sub>, …, x<sub>n</sub>_ are parameters we are trying to find,
/// - _n_ is the number of parameters.
///
/// Constraints are defined as
/// _b<sub>j1</sub>x<sub>1</sub> + … + b<sub>jm</sub>x<sub>n</sub> ≤ b<sub>j0</sub>_ for _j=1,…,m_
/// where
/// - _b<sub>j0</sub>, …, b<sub>jn</sub>_ are known coefficients of constraint equation _j_,
/// - _m_ is the number of the constraint equations.
///
/// # Arguments
///
/// - `objective_lhs`: coefficients _a<sub>1</sub>…a<sub>n</sub>_ of the objective function.
/// - `objective_rhs`: free-standing coefficient _a<sub>0</sub>_ of the objective function.
/// - `constraints_lhs`: coefficients _b<sub>j1</sub>, …, b<sub>jn</sub>_ for _j=1,…,m_ of the constraints equations in row-major order.
/// - `constraints_rhs`: free-standing coefficnets _b<sub>j0</sub>_ of the constraints equations.
///
/// # Return value
///
/// The resulting parameters _x<sub>1</sub>, …, x<sub>n</sub>_ are returned in `constraints_rhs`,
/// the resulting objective function value is returned in `objective_rhs`.
///
/// Error is returned if right-hand side is negative.
///
/// # Example
///
/// Maximize _Z = 3x<sub>1</sub> + 2x<sub>2</sub>_ subject to the following constraints: \
/// _2x<sub>1</sub> + x<sub>2</sub> ≤ 18_ \
/// _2x<sub>1</sub> + 3x<sub>2</sub> ≤ 42_
///
/// ```rust
/// let mut constraints_rhs = [18.0, 42.0];
/// let mut objective_rhs = 0.0;
/// lprog::maximize(
///     &mut [-3.0, -2.0, 0.0, 0.0],
///     &mut objective_rhs,
///     &mut [
///         2.0, 1.0, 1.0, 0.0, //
///         2.0, 3.0, 0.0, 1.0, //
///     ],
///     &mut constraints_rhs,
/// )
/// .unwrap();
/// let params = constraints_rhs;
/// let objective_value = objective_rhs;
/// assert_eq!([3.0, 12.0], params);
/// assert_eq!(33.0, objective_value);
/// ```
pub fn maximize(
    objective_lhs: &mut [f64],
    objective_rhs: &mut f64,
    constraints_lhs: &mut [f64],
    constraints_rhs: &mut [f64],
) -> Result<(), MaximizeError> {
    if constraints_lhs.is_empty() && constraints_rhs.is_empty() {
        return Ok(());
    }
    let column_count = objective_lhs.len();
    assert_eq!(0, constraints_lhs.len() % column_count);
    assert_eq!(constraints_rhs.len(), constraints_lhs.len() / column_count);
    loop {
        let Some((pivot_column_index, _)) = objective_lhs
            .iter()
            .copied()
            .enumerate()
            .filter(|(_i, value)| *value < 0.0)
            .min_by(|(_i, a), (_j, b)| a.total_cmp(b))
        else {
            // No negative coefficients left.
            break;
        };
        let pivot_row_index = constraints_lhs
            .chunks_exact(column_count)
            .zip(constraints_rhs.iter().copied())
            .enumerate()
            .map(|(i, (lhs_row, constraints_rhs))| {
                (i, constraints_rhs / lhs_row[pivot_column_index])
            })
            .filter(|(_i, value)| *value > 0.0)
            .min_by(|(_i, a), (_j, b)| a.total_cmp(b))
            .ok_or(MaximizeError)?
            .0;
        // Update pivot row.
        let (lhs_pre, rest) = constraints_lhs.split_at_mut(column_count * pivot_row_index);
        let (pivot_row, lhs_post) = rest.split_at_mut(column_count);
        let factor = pivot_row[pivot_column_index];
        for value in pivot_row.iter_mut() {
            *value /= factor;
        }
        constraints_rhs[pivot_row_index] /= factor;
        let pivot_rhs = constraints_rhs[pivot_row_index];
        // Update other rows.
        for (lhs_row, rhs_elem) in lhs_pre
            .chunks_exact_mut(column_count)
            .zip(constraints_rhs.iter_mut())
        {
            let factor = lhs_row[pivot_column_index];
            for (value, pivot_value) in lhs_row.iter_mut().zip(pivot_row.iter().copied()) {
                *value -= factor * pivot_value;
            }
            *rhs_elem -= factor * pivot_rhs;
        }
        for (lhs_row, rhs_elem) in lhs_post
            .chunks_exact_mut(column_count)
            .zip(constraints_rhs.iter_mut().skip(pivot_row_index + 1))
        {
            let factor = lhs_row[pivot_column_index];
            for (value, pivot_value) in lhs_row.iter_mut().zip(pivot_row.iter().copied()) {
                *value -= factor * pivot_value;
            }
            *rhs_elem -= factor * pivot_rhs;
        }
        // Update objective row.
        {
            let factor = objective_lhs[pivot_column_index];
            for (value, pivot_value) in objective_lhs.iter_mut().zip(pivot_row.iter().copied()) {
                *value -= factor * pivot_value;
            }
            *objective_rhs -= factor * pivot_rhs;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximize_works() {
        // Maximize z = 3*x1 + 2*x2
        // 2*x1 + x2 <= 18
        // 2*x1 + 3*x2 <= 42
        // x1 >= 0
        // x2 >= 0
        let mut constraints_rhs = [18.0, 42.0];
        let mut objective_rhs = 0.0;
        maximize(
            &mut [-3.0, -2.0, 0.0, 0.0],
            &mut objective_rhs,
            &mut [
                2.0, 1.0, 1.0, 0.0, //
                2.0, 3.0, 0.0, 1.0, //
            ],
            &mut constraints_rhs,
        )
        .unwrap();
        assert!(
            constraints_rhs
                .iter()
                .copied()
                .zip([3.0, 12.0])
                .map(|(a, b)| (a - b).abs())
                .all(|abs_diff| abs_diff < 1e-3),
            "constraints_rhs = {constraints_rhs:?}"
        );
        assert!(
            (objective_rhs - 33.0).abs() < 1e-3,
            "objective_rhs = {objective_rhs:?}"
        );
    }
}
