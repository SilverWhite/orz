#!/usr/bin/env bash
# Official Terminal-Bench 2.1 run — chunked variant (k=1, 10-task chunks).
#
# Compliant settings only (leaderboard static-analysis requirements):
#   - dataset pinned to the exact ref the CI checks against
#   - default execution settings: timeout_multiplier None/1.0, no agent or
#     verifier timeout overrides, no resource overrides (no --ak max_wallclock
#     is passed, so every task uses its full official agent timeout)
#   - --upload --public so trials land on Harbor Hub and are publicly readable
#
# Structure: 5 rounds x 89 tasks, k=1 per task. One attempt per task per
# round = 5 attempts/task total (the official minimum, MIN_TRIALS_PER_TASK=5).
# Each round is split into 10-task chunks (c1..c9; the last chunk has 9).
#
#   bash run_official_chunked_10.sh 1     # round 1 only (9 jobs, 89 attempts)
#   bash run_official_chunked_10.sh all   # rounds 1..5 sequentially
#   bash run_official_chunked_10.sh 1 1   # round 1, chunk 1 only (10 tasks)
#
# A chunk whose job directory already contains result.json is skipped, so a
# failed run can be resumed by re-invoking the same round.
#
# Merge into one submission afterwards (all jobs share the agent+model key,
# so they merge into a single file):
#   cd D:/tb-eval/terminal-bench-2-1/leaderboard
#   uv run lb filter <hub job link or uuid for every official-r*-c* job>
#
# Preconditions:
#   1. `harbor auth login` (--upload requires Harbor Hub auth)
#   2. ORZ_DEEPSEEK_API_KEY present in D:/tb-eval/.env
#   3. Output goes to D:/tb-eval/jobs-official
set -euo pipefail
export PYTHONPATH=D:/tb-eval
HARBOR="D:/tb-eval/venv/Scripts/harbor.exe"
ORZ="D:/tb-eval/orz-linux/orz"
DATASET="terminal-bench/terminal-bench-2-1@sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a"
MODEL="deepseek-v4-flash"
VOL_ROOT="D:/tb-eval/gsa-volumes"
JOBS_DIR="D:/tb-eval/jobs-official"
CHUNK=10

TASKS=(build-pov-ray schemelike-metacircular-eval llm-inference-batching-scheduler feal-linear-cryptanalysis dna-assembly feal-differential-cryptanalysis polyglot-c-py qemu-startup sqlite-db-truncate vulnerable-secret build-cython-ext configure-git-webserver fix-git headless-terminal merge-diff-arc-agi-task git-multibranch sam-cell-seg portfolio-optimization video-processing mcmc-sampling-stan path-tracing-reverse mteb-retrieve code-from-image break-filter-js-from-html sanitize-git-repo sparql-university tune-mjcf git-leak-recovery cobol-modernization fix-code-vulnerability gpt2-codegolf log-summary-date-ranges openssl-selfsigned-cert mteb-leaderboard reshard-c4-data winning-avg-corewars caffe-cifar-10 rstan-to-pystan extract-moves-from-video custom-memory-heap-crash constraints-scheduling pytorch-model-recovery prove-plus-comm raman-fitting torch-pipeline-parallelism adaptive-rejection-sampler cancel-async-tasks db-wal-recovery password-recovery kv-store-grpc multi-source-data-merger modernize-scientific-stack install-windows-3.11 fix-ocaml-gc train-fasttext circuit-fibsqrt path-tracing mailman crack-7z-hash large-scale-text-editing pytorch-model-cli pypi-server regex-log torch-tensor-parallelism make-doom-for-mips chess-best-move extract-elf write-compressor largest-eigenval nginx-request-logging regex-chess distribution-search bn-fit-modify compile-compcert make-mips-interpreter filter-js-from-html dna-insert protein-assembly financial-document-processor polyglot-rust-c query-optimize sqlite-with-gcov qemu-alpine-ssh build-pmars count-dataset-tokens gcode-to-text hf-model-inference model-extraction-relu-logits overfull-hbox)

run_chunk() {
  local job_name="$1"; shift
  local vol_dir="${VOL_ROOT}/${job_name}"
  if [ -f "${JOBS_DIR}/${job_name}/result.json" ]; then
    local json
    json="$(<"${JOBS_DIR}/${job_name}/result.json")"
    # Only a finished job has a non-null finished_at ("finished_at": "<iso>").
    if [[ "$json" == *'"finished_at": "'* ]]; then
      echo "==> ${job_name} already complete (finished result.json present), skipping"
      return 0
    fi
  fi
  mkdir -p "$vol_dir"
  local mounts="[{\"type\":\"bind\",\"source\":\"${vol_dir}\",\"target\":\"/orz-gsa\"}]"
  local args=()
  for t in "$@"; do
    args+=( -i "terminal-bench/$t" )
  done
  echo "==> ${job_name} : $# tasks | volume=${vol_dir}"
  "$HARBOR" run \
    -d "$DATASET" \
    "${args[@]}" \
    -a tb_agents.orz:Orz \
    -m "$MODEL" \
    --ak "orz_binary=$ORZ" \
    --ak "model_id=$MODEL" \
    --ak "gsa_volume=$vol_dir" \
    --mounts "$mounts" \
    --env-file D:/tb-eval/.env \
    --job-name "$job_name" \
    -o "$JOBS_DIR" \
    -k 1 \
    --upload \
    --public \
    -y
}

run_round() {
  local round="$1"
  local n=${#TASKS[@]}
  local i=0 c=1
  echo "===== round $round : $n tasks x k=1, chunks of $CHUNK ====="
  while [ "$i" -lt "$n" ]; do
    local chunk=("${TASKS[@]:i:CHUNK}")
    run_chunk "official-r${round}-c${c}" "${chunk[@]}"
    i=$((i+CHUNK)); c=$((c+1))
  done
}

AUTH_STATUS="$("$HARBOR" auth status 2>&1)"
if [[ "$AUTH_STATUS" == *"Not authenticated"* ]]; then
  echo "ERROR: Harbor Hub not authenticated. Run: harbor auth login" >&2
  exit 1
fi

case "${1:-all}" in
  all)
    for r in 1 2 3 4 5; do
      run_round "$r"
    done
    ;;
  1|2|3|4|5)
    if [ -n "${2:-}" ]; then
      start=$(( (${2} - 1) * CHUNK ))
      chunk=("${TASKS[@]:start:CHUNK}")
      if [ "${#chunk[@]}" -eq 0 ]; then
        echo "ERROR: invalid chunk ${2} for round ${1}" >&2
        exit 2
      fi
      run_chunk "official-r${1}-c${2}" "${chunk[@]}"
    else
      run_round "$1"
    fi
    ;;
  *)
    echo "usage: $0 [1|2|3|4|5|all]" >&2
    exit 2
    ;;
esac
