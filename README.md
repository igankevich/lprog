# lprog

[![Crates.io Version](https://img.shields.io/crates/v/lprog)](https://crates.io/lprog/lprog)
[![Docs](https://docs.rs/lprog/badge.svg)](https://docs.rs/lprog)
[![dependency status](https://deps.rs/repo/github/igankevich/lprog/status.svg)](https://deps.rs/repo/github/igankevich/lprog)

This crate implements constraint optimization using linear programming (Simplex method).
Both minimization and maximization of the objective function is supported.
Constraint can be any linear equation or non-strict inequality.

Objective function is defined as
_f(a<sub>0</sub>, …, a<sub>n</sub>) = a<sub>0</sub> + a<sub>1</sub>x<sub>1</sub> + … + a<sub>n</sub>x<sub>n</sub>_ where
- _a<sub>0</sub>, …, a<sub>n</sub>_ are known coefficients of the objective function,
- _x<sub>1</sub>, …, x<sub>n</sub>_ are parameters we are trying to find,
- _n_ is the number of parameters.

Constraints are defined as
_b<sub>j1</sub>x<sub>1</sub> + … + b<sub>jm</sub>x<sub>n</sub> ≤ b<sub>j0</sub>_ for _j=1,…,m_
where
- _b<sub>j0</sub>, …, b<sub>jn</sub>_ are known coefficients of constraint equation _j_,
- _m_ is the number of the constraint equations.

Instead of `≤` either `≥` or `=` can be used.
