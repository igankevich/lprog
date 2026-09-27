use alloc::borrow::Cow;
use alloc::vec;
use alloc::vec::Vec;

use crate::MaximizeError;
use crate::maximize;

/// Objective kind.
#[derive(Debug)]
pub enum ObjectiveKind {
    /// Maximize the objective function.
    Maximize,
    /// Minimize the objective function.
    Minimize {
        /// "Infinity" number.
        ///
        /// This number is used to penalize artificial variables. Should at least be larger than
        /// any of the coefficients. Use any large value that will not lead to overflows or large
        /// numerical errors.
        m: f64,
    },
}

/// Objective function.
#[derive(Debug)]
pub struct Objective<'a> {
    pub kind: ObjectiveKind,
    /// Per-parameter coefficients.
    pub lhs: &'a [f64],
    /// Free-standing coefficient.
    pub rhs: f64,
}

/// Constraint kind.
#[derive(Debug)]
pub enum ConstraintKind {
    /// Exact equation.
    Equal,
    /// Greater-than-or-equal inequality.
    GreaterOrEqual,
    /// Less-than-or-equal inequality.
    LessOrEqual,
}

/// Optimization constraint in natural (potentially non-standard) form.
#[derive(Debug)]
pub struct Constraint<'a> {
    pub kind: ConstraintKind,
    /// Equation/inequality coefficients.
    pub lhs: Cow<'a, [f64]>,
    /// Right-hand side of the equation/inequality.
    pub rhs: f64,
}

impl<'a> Constraint<'a> {
    pub fn new(lhs: impl Into<Cow<'a, [f64]>>, kind: ConstraintKind, rhs: f64) -> Self {
        let lhs = lhs.into();
        Self { kind, lhs, rhs }
    }
}

/// A matrix form of objective function and constraints suitable for supplying to
/// [`maximize`](crate::maximize).
#[derive(Debug)]
pub struct StandardForm {
    pub objective_lhs: Vec<f64>,
    pub objective_rhs: f64,
    pub constraints_lhs: Vec<f64>,
    pub constraints_rhs: Vec<f64>,
}

/// Convert objective function and constraints to standard form.
///
/// Standard form includes
/// - slack variables that model lesser-than-or-equal constraints,
/// - artificial variables that model greater-than-or-equal constraints,
/// - modified coefficients to handle minimization.
///
/// After conversion the result can be fed to [`maximize`](crate::maximize).
///
/// Note that if you are minimizing, then the parameters that `maximize` outputs need to be
/// truncated and the sign of the objective function value need to be flipped.
/// Consider using [`optimize`] to do that automatically.
pub fn to_standard_form(objective: Objective, constraints: &[Constraint]) -> StandardForm {
    let regular_variable_count = objective.lhs.len();
    let mut slack_variable_count = 0;
    let mut surplus_variable_count = 0;
    for Constraint { lhs, kind, .. } in constraints.iter() {
        assert_eq!(lhs.len(), regular_variable_count);
        match kind {
            ConstraintKind::Equal => {}
            ConstraintKind::LessOrEqual => slack_variable_count += 1,
            ConstraintKind::GreaterOrEqual => surplus_variable_count += 1,
        }
    }
    let column_count = regular_variable_count + slack_variable_count + surplus_variable_count;
    let row_count = constraints.len();
    let mut lhs = vec![0.0; column_count * row_count];
    let mut rhs = vec![0.0; row_count];
    let mut objective_lhs = vec![0.0; column_count];
    let mut objective_rhs = 0.0;
    let mut slack_variable_index = 0;
    let mut surplus_variable_index = 0;
    for ((lhs_row, rhs_elem), constr) in lhs
        .chunks_exact_mut(column_count)
        .zip(rhs.iter_mut())
        .zip(constraints.iter())
    {
        *rhs_elem = constr.rhs;
        lhs_row[..regular_variable_count].copy_from_slice(&constr.lhs);
        match constr.kind {
            ConstraintKind::Equal => {}
            ConstraintKind::LessOrEqual => {
                let value = match objective.kind {
                    ObjectiveKind::Maximize => 1.0,
                    ObjectiveKind::Minimize { .. } => -1.0,
                };
                lhs_row[regular_variable_count + slack_variable_index] = value;
                slack_variable_index += 1;
            }
            ConstraintKind::GreaterOrEqual => {
                lhs_row[regular_variable_count + slack_variable_count + surplus_variable_index] =
                    1.0;
                objective_rhs += *rhs_elem;
                for (last, cur) in objective_lhs
                    .iter_mut()
                    .take(regular_variable_count)
                    .zip(lhs_row.iter().take(regular_variable_count).copied())
                {
                    *last += cur;
                }
                surplus_variable_index += 1;
            }
        }
    }
    match objective.kind {
        ObjectiveKind::Maximize => {
            for (value, obj) in objective_lhs
                .iter_mut()
                .take(regular_variable_count)
                .zip(objective.lhs.iter().copied())
            {
                *value = -obj;
            }
            objective_rhs = objective.rhs;
        }
        ObjectiveKind::Minimize { m } => {
            for (value, obj) in objective_lhs
                .iter_mut()
                .take(regular_variable_count)
                .zip(objective.lhs.iter().copied())
            {
                *value = obj - m * (*value);
            }
            objective_lhs[regular_variable_count..][..slack_variable_count].fill(m);
            objective_rhs = objective.rhs - m * objective_rhs;
        }
    }
    StandardForm {
        objective_lhs,
        objective_rhs,
        constraints_lhs: lhs,
        constraints_rhs: rhs,
    }
}

/// Optimize objective function while satisfying constraints.
///
/// This function takes care of converting objective function and constraints to standard form via
/// [`to_standard_form`] and then post-processing the outputs of [`maximize`](crate::maximize) to
/// match the original form.
///
/// Returns the resuling parameters and the resulting objective function value.
///
/// This function is a wrapper around [`maximize`](crate::maximize).
/// See its documentation for further information.
///
/// # Example
///
/// Minimize _Z = 3x<sub>1</sub> + 4x<sub>2</sub>_ subject to the following constraints: \
/// _2x<sub>1</sub> + x<sub>2</sub> ≥ 8_ \
/// _x<sub>1</sub> + 2x<sub>2</sub> ≥ 10_
///
/// ```rust
/// use lprog::{Constraint, ConstraintKind::*, Objective, ObjectiveKind::*};
/// let (params, objective_value) = lprog::optimize(
///     Objective { kind: Minimize { m: 1e3 }, lhs: &[3.0, 4.0], rhs: 0.0 },
///     &[
///         Constraint::new(&[2.0, 1.0], GreaterOrEqual, 8.0),
///         Constraint::new(&[1.0, 2.0], GreaterOrEqual, 10.0),
///     ],
/// )
/// .unwrap();
/// assert_eq!([2.0, 4.0].as_slice(), params.as_slice());
/// assert_eq!(22.0, objective_value);
/// ```
pub fn optimize(
    objective: Objective,
    constraints: &[Constraint],
) -> Result<(Vec<f64>, f64), MaximizeError> {
    let regular_variable_count = objective.lhs.len();
    let is_minimize = matches!(objective.kind, ObjectiveKind::Minimize { .. });
    let StandardForm {
        mut objective_lhs,
        mut objective_rhs,
        mut constraints_lhs,
        mut constraints_rhs,
    } = to_standard_form(objective, constraints);
    maximize(
        &mut objective_lhs[..],
        &mut objective_rhs,
        &mut constraints_lhs[..],
        &mut constraints_rhs[..],
    )?;
    constraints_rhs.truncate(regular_variable_count);
    if is_minimize {
        objective_rhs = -objective_rhs;
    }
    Ok((constraints_rhs, objective_rhs))
}

/// Fit linear model into the observations while ensuring that the resulting model never
/// underpredicts but might overpredict.
///
/// Note that you need a dummy variable for a free-standing coefficient of the model.
/// See the example.
///
/// # Example
///
/// ```rust
/// let linear_model = |a, b, c| 10.0 * a + 20.0 * b + 30.0 * c;
/// let (params, objective_value) = lprog::fit(
///     3,
///     [
///         (&[1.0, 0.0, 0.0], linear_model(1.0, 0.0, 0.0)),
///         (&[1.0, 1.0, 0.0], linear_model(1.0, 1.0, 0.0)),
///         (&[1.0, 0.0, 1.0], linear_model(1.0, 0.0, 1.0)),
///         (&[1.0, 1.0, 1.0], linear_model(1.0, 1.0, 1.0)),
///     ],
/// )
/// .unwrap();
/// assert_eq!([10.0, 20.0, 30.0].as_slice(), params.as_slice());
/// assert_eq!(0.0, objective_value);
/// ```
pub fn fit<'a>(
    param_count: usize,
    observations: impl IntoIterator<Item = (impl Into<Cow<'a, [f64]>>, f64)>,
) -> Result<(Vec<f64>, f64), MaximizeError> {
    let mut objective_lhs = vec![0.0; param_count];
    let mut objective_rhs = 0.0;
    let observations = observations.into_iter();
    let (min_size, max_size) = observations.size_hint();
    let mut constraints = Vec::with_capacity(max_size.unwrap_or(min_size));
    for (x, f) in observations {
        let x = x.into();
        assert_eq!(x.len(), param_count);
        for (objective_lhs_i, x_i) in objective_lhs.iter_mut().zip(x.iter().copied()) {
            *objective_lhs_i += x_i;
        }
        objective_rhs += f;
        constraints.push(Constraint {
            lhs: x,
            kind: ConstraintKind::GreaterOrEqual,
            rhs: f,
        });
    }
    optimize(
        Objective {
            kind: ObjectiveKind::Minimize { m: 1e3 },
            lhs: &objective_lhs,
            rhs: objective_rhs,
        },
        &constraints,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optimize_works() {
        // Maximize z = 3*x1 + 2*x2
        // 2*x1 + x2 <= 18
        // 2*x1 + 3*x2 <= 42
        // x1 >= 0
        // x2 >= 0
        let (rhs, objective_rhs) = optimize(
            Objective {
                kind: ObjectiveKind::Maximize,
                lhs: &[3.0, 2.0],
                rhs: 0.0,
            },
            &[
                Constraint {
                    lhs: Cow::Borrowed(&[2.0, 1.0]),
                    kind: ConstraintKind::LessOrEqual,
                    rhs: 18.0,
                },
                Constraint {
                    lhs: Cow::Borrowed(&[2.0, 3.0]),
                    kind: ConstraintKind::LessOrEqual,
                    rhs: 42.0,
                },
            ],
        )
        .unwrap();
        assert!(
            rhs.iter()
                .copied()
                .zip([3.0, 12.0])
                .map(|(a, b)| (a - b).abs())
                .all(|abs_diff| abs_diff < 1e-3),
            "rhs = {rhs:?}"
        );
        assert!(
            (objective_rhs - 33.0).abs() < 1e-3,
            "objective_rhs = {objective_rhs:?}"
        );
    }

    #[test]
    fn minimize_works() {
        // Minimize w = 3*x1 + 4*x2
        // 2*x1 + x2 >= 8
        // x1 + 2*x2 >= 10
        // x1 >= 0
        // x2 >= 0
        let (rhs, objective_rhs) = optimize(
            Objective {
                kind: ObjectiveKind::Minimize { m: 1e3 },
                lhs: &[3.0, 4.0],
                rhs: 0.0,
            },
            &[
                Constraint {
                    lhs: Cow::Borrowed(&[2.0, 1.0]),
                    kind: ConstraintKind::GreaterOrEqual,
                    rhs: 8.0,
                },
                Constraint {
                    lhs: Cow::Borrowed(&[1.0, 2.0]),
                    kind: ConstraintKind::GreaterOrEqual,
                    rhs: 10.0,
                },
            ],
        )
        .unwrap();
        assert!(
            rhs.iter()
                .copied()
                .zip([2.0, 4.0])
                .map(|(a, b)| (a - b).abs())
                .all(|abs_diff| abs_diff < 1e-3),
            "rhs = {rhs:?}"
        );
        assert!(
            (objective_rhs - 22.0).abs() < 1e-3,
            "objective_rhs = {objective_rhs:?}"
        );
    }

    #[test]
    fn fit_works() {
        let expected_model = |a, b, c| 10.0 * a + 20.0 * b + 30.0 * c;
        let (rhs, objective_rhs) = fit(
            3,
            [
                (&[1.0, 0.0, 0.0], expected_model(1.0, 0.0, 0.0)),
                (&[1.0, 1.0, 0.0], expected_model(1.0, 1.0, 0.0)),
                (&[1.0, 0.0, 1.0], expected_model(1.0, 0.0, 1.0)),
                (&[1.0, 1.0, 1.0], expected_model(1.0, 1.0, 1.0)),
            ],
        )
        .unwrap();
        assert!(
            rhs.iter()
                .copied()
                .zip([10.0, 20.0, 30.0])
                .map(|(a, b)| (a - b).abs())
                .all(|abs_diff| abs_diff < 1e-3),
            "rhs = {rhs:?}"
        );
        assert!(
            (objective_rhs - 0.0).abs() < 1e-3,
            "objective_rhs = {objective_rhs:?}"
        );
    }
}
