#!/bin/sh
# ORZ Linux musl 载体构建（392 批复原脚本）。
# 原 build_orz_aliyun_trixie.sh 随 0du 目录清理灭失；本脚本按 052/346/357
# 三批档在案口径复原：rust:1.97-slim（trixie）容器内构建、musl static-PIE
# 三件套（orz／orz-signer／orz-acaf-provision）、apt 走阿里云镜像。
# 调用契约（ORZ-BUILD-MOUNT-001 + MSYS_NO_PATHCONV=1）：
#   docker run --rm \
#     -v D:\CLI\orz:/orz/orz \
#     -v D:\tb-eval\orz-linux:/orz/out \
#     -v D:\tb-eval\orz-linux-target:/orz/target \
#     -v D:\tb-eval\orz-linux-cargo:/orz/cargo \
#     -e CARGO_HOME=/orz/cargo -e CARGO_TARGET_DIR=/orz/target \
#     -w /orz/orz rust:1.97-slim sh /orz/orz/build_orz_aliyun_trixie.sh
set -e

# trixie deb822 源换阿里云（052 批 apt 连败先例；直连可用时无损）。
cat > /etc/apt/sources.list.d/debian.sources <<'EOF'
Types: deb
URIs: https://mirrors.aliyun.com/debian/
Suites: trixie
Components: main
Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg
EOF
apt-get update -qq
# musl-tools＝musl 目标链接；protobuf-compiler＝orz-tools-api build.rs 的
# protoc（仓内只 vendor Windows 版 bin/protoc.exe，容器内不可执行——
# 392 批首轮构建实证后补入）。
apt-get install -y -qq musl-tools protobuf-compiler > /dev/null

# musl target（工具链版本随 rust:1.97-slim 镜像冻结，与 346/357 同基）。
rustup target add x86_64-unknown-linux-musl

cargo build -p orz-bin --release --target x86_64-unknown-linux-musl

mkdir -p /orz/out
for b in orz orz-signer orz-acaf-provision; do
  cp "$CARGO_TARGET_DIR/x86_64-unknown-linux-musl/release/$b" "/orz/out/$b"
done
echo "LINUX_BUILD_EXIT=0"
ls -l /orz/out
