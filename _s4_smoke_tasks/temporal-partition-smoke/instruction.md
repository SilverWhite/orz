Before doing anything else, inspect the temporal partition of the blackboard with the blackboard_read tool. Make exactly these four calls:

1. blackboard_read with section="temporal", selector="now"
2. blackboard_read with section="temporal", selector="recent", k=5
3. blackboard_read with section="temporal", selector="history", k=5
4. blackboard_read with section="temporal", selector="feature", name="u_prog", k=5

Briefly note in your working notes what each returned render shows (the rows contain fields such as u_prog=, u_err=, u_stuck=, err10=, succ10=, T̂= and a domain label).

Then complete this trivial task: write the exact string "temporal-smoke-ok-2026-08-31" to the file /app/answer.txt. Do not modify any other files.
