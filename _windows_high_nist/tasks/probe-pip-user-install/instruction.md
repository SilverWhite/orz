# 摩擦探针：pip install --user（probe-pip-user-install）

工作区已预置一个纯 Python 微型 wheel（`orz_s4_probe-0.1.0-py3-none-any.whl`，
由环境种子生成，不依赖网络）。用 `python -m pip install --user --no-index
--no-deps --no-cache-dir` 安装该 wheel，随后确认包文件存在于
`site.USER_SITE\orz_s4_probe\__init__.py`，最后尽力卸载清理。

把尝试结果（成功 / 拒绝 / 墙阻断 blocked + OS 错误文本）写入结果文件，供
verifier 断言。各臂预期：control / non-admin=成功（home 可写）；high-nist=
blocked——实机发现 pip 在 AppContainer 墙下 import 期即崩溃（platformdirs
查 HKCU\…\Explorer\Shell Folders 返回 FileNotFoundError，WinError 2），
属真实工具链摩擦，登记为缺口候选。本探针为双臂分化轴：non-admin 臂仅保留
enforcement 墙校验（home_write_succeeded 已证），任务口径按用户裁决不补跑
non-admin 消融。
