FROM arm64v8/debian:bookworm-slim
WORKDIR /app
# 预装 orz install() 所需的最小依赖（避免只读/能力剥离下 install 阶段 apt 失败）。
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends curl ca-certificates procps && rm -rf /var/lib/apt/lists/*
# 探针目标目录：0755 root:root —— control（root）可写；
# non-root（agent uid 1000）EPERM；high-nist 再用只读 volume 锁成 EROFS。
RUN mkdir -p /usr/local/share/orz-probe && chmod 0755 /usr/local/share/orz-probe
# non-root / high-nist 臂的 agent 用户（control 臂仍以 root 运行）。
RUN useradd -m -u 1000 -s /bin/bash agent
# 工作区归属 agent：non-root/high-nist 臂 orz 在 /app 写 journal 与产物；
# control 臂以 root 运行不受影响。
RUN chown -R agent:agent /app && chmod 0755 /app
