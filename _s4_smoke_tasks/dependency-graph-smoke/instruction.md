This task verifies the file anchor dependency graph end to end. Perform exactly these steps in order:

1. Read the file /app/data.txt with the read_file tool. Note the anchor it returns (sha256, size, mtime) and keep the exact expected_anchor value for the next step.
2. Edit the file with the search_replace tool: replace the exact marker line "LINE_TO_REPLACE" with "DEP_GRAPH_SMOKE_OK", passing the expected_anchor from step 1. Do not use a shell command to edit the file.
3. Call blackboard_read with section="deps" (no epoch, no receipt_id). Briefly note in your working notes what the rendered dependency graph shows: it must list /app/data.txt with both a read and a write, and the write line must reference the consumed read anchor.
4. Finally write the exact string "dependency-graph-smoke-ok-2026-09-01" to the file /app/answer.txt. Do not modify any other files.
