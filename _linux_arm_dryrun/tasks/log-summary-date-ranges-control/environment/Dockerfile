FROM arm64v8/debian:bookworm-slim
WORKDIR /app
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends python3 curl ca-certificates procps && rm -rf /var/lib/apt/lists/*
RUN useradd -m -u 1000 -s /bin/bash agent && chown -R agent:agent /app && chmod 0755 /app
COPY log_generator_deterministic.py /tmp/log_generator_deterministic.py
# 镜像构建期生成种子日志：三臂同数据（幂等，同一 Dockerfile 构建同一层）。
RUN python3 /tmp/log_generator_deterministic.py && rm /tmp/log_generator_deterministic.py && chown -R agent:agent /app/logs
