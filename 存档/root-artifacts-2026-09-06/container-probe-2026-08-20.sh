#!/usr/bin/env bash
# Container-side reproduction probe (2026-08-20): run the SAME orz binary,
# prompt and flags as the eval harness inside the gpt2-codegolf image, with
# tcpdump capturing the api.deepseek.com traffic and unbuffered logs written
# through the host bind mount.
set -u

KEY="$(grep '^ORZ_DEEPSEEK_API_KEY=' /env/.env | head -1 | cut -d= -f2- | tr -d '\r')"
if [ -z "$KEY" ]; then
  echo KEY-MISSING
  exit 1
fi
export ORZ_DEEPSEEK_API_KEY="$KEY"
export ORZ_ACAF_FAIL_CLOSED=0

apt-get update -qq >/dev/null 2>&1
apt-get install -y -qq tcpdump >/dev/null 2>&1

mkdir -p /probe
mkdir -p /probe/gsa
# Replicate the eval's journal path: /app/.gsa -> host bind mount.
if [ ! -e /app/.gsa ]; then
  ln -s /probe/gsa /app/.gsa
fi
cd /app || exit 1

(tcpdump -i any -s 0 -w /probe/traffic.pcap 'tcp port 443' >/dev/null 2>&1 &)
sleep 2

date -Is > /probe/start.txt
EXEC_PREFIX=""
if command -v stdbuf >/dev/null 2>&1; then
  EXEC_PREFIX="stdbuf -oL -eL"
fi
$EXEC_PREFIX /orz-bin/orz \
  -p 'I have downloaded the gpt-2 weights stored as a TF .ckpt. Write me a dependency-free C file that samples from the model with arg-max sampling. Call your program /app/gpt2.c, I will compile with gcc -O3 -lm. It should read the .ckpt and the .bpe file. Your c program must be <5000 bytes. I will run it /app/a.out gpt2-124M.ckpt vocab.bpe "[input string here]" and you should continue the output under whatever GPT-2 would print for the next 20 tokens.' \
  --real --allow-write --allow-shell --max-tool-rounds 999 --max-wallclock 1740 --allow-network \
  > /probe/orz-container-probe.log 2>&1
STATUS=$?
echo "EXIT=$STATUS" >> /probe/orz-container-probe.log
date -Is > /probe/end.txt
sleep 2
pkill tcpdump 2>/dev/null || true
tail -8 /probe/orz-container-probe.log
