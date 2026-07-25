# Synthetic exponential-decay fixture

This checked-in fixture studies the initial-value problem
`dy/dt = -y`, `y(0) = 1` at `t = 1`.

Two precomputed explicit-Euler runs use step sizes `0.1` and `0.05`.
The exact reference value is `exp(-1)`. The narrow review question is
whether the smaller step has a smaller final absolute error in these two
specific runs.

This fixture is synthetic, independent of LIF, and does not establish
convergence order, causal mechanism, generality, or production readiness.
