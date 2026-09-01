FROM arm64v8/debian:bookworm-slim
WORKDIR /app
# 预装 orz install() 所需的最小依赖（能力剥离环境下 install 阶段不再 apt）。
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends curl ca-certificates procps && rm -rf /var/lib/apt/lists/*
# non-root / high-nist 臂的 agent 用户（control 臂仍以 root 运行）。
RUN useradd -m -u 1000 -s /bin/bash agent
# 工作区归属 agent：non-root/high-nist 臂 orz 在 /app 写 journal 与产物。
RUN chown -R agent:agent /app && chmod 0755 /app
