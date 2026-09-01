#!/usr/bin/env bash
# THIN-HARNESS-REDESIGN R1/R2 编译轮后失败题重跑（2026-08-28）——
# 上次 89 题官方跑分（official-r1，k=1，58/89）未解出的 31 题，
# 用新二进制（orz 6cc8586，S3 重建 07:00 HKT）k=1 再跑一轮，10 题一批。
#
# 结构与 run_official_chunked_10.sh 一致（chunked variant, k=1, 10-task
# chunks），但任务表=31 道错题、job 前缀=official-r2-failures，且**不上传**
# （本地验证重跑，leaderboard 合并另行决定）。
#
#   bash run_failures_chunked_10.sh 1     # chunk 1 only (10 tasks)
#   bash run_failures_chunked_10.sh all   # chunks 1..4 sequentially
#
# 已含 result.json（finished_at 非空）的 job 自动跳过，可断点续跑。
#
# Preconditions:
#   1. ORZ_DEEPSEEK_API_KEY present in D:/tb-eval/.env
#   2. D:/tb-eval/orz-linux/orz = 新二进制（S3 重建产物）
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

TASKS=(query-optimize chess-best-move make-doom-for-mips model-extraction-relu-logits pytorch-model-cli torch-pipeline-parallelism raman-fitting adaptive-rejection-sampler write-compressor largest-eigenval gpt2-codegolf tune-mjcf count-dataset-tokens gcode-to-text extract-elf caffe-cifar-10 filter-js-from-html make-mips-interpreter protein-assembly dna-insert path-tracing rstan-to-pystan extract-moves-from-video path-tracing-reverse mteb-retrieve dna-assembly circuit-fibsqrt mteb-leaderboard video-processing train-fasttext build-pov-ray)

run_chunk() {
  local job_name="$1"; shift
  local vol_dir="${VOL_ROOT}/${job_name}"
  if [ -f "${JOBS_DIR}/${job_name}/result.json" ]; then
    local json
    json="$(<"${JOBS_DIR}/${job_name}/result.json")"
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
    -y
}

run_round() {
  local n=${#TASKS[@]}
  local i=0 c=1
  echo "===== failures rerun : $n tasks x k=1, chunks of $CHUNK ====="
  while [ "$i" -lt "$n" ]; do
    local chunk=("${TASKS[@]:i:CHUNK}")
    run_chunk "official-r2-failures-c${c}" "${chunk[@]}"
    i=$((i+CHUNK)); c=$((c+1))
  done
}

case "${1:-all}" in
  all)
    run_round
    ;;
  [1-4])
    local start=$(( (${1} - 1) * CHUNK ))
    local chunk=("${TASKS[@]:start:CHUNK}")
    if [ "${#chunk[@]}" -eq 0 ]; then
      echo "ERROR: invalid chunk ${1}" >&2
      exit 2
    fi
    run_chunk "official-r2-failures-c${1}" "${chunk[@]}"
    ;;
  *)
    echo "usage: $0 [1|2|3|4|all]" >&2
    exit 2
    ;;
esac
