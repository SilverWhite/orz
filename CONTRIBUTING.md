# Contributing

This repository does **not** accept external pull requests or unsolicited
patches.

SpaceXAI develops this software internally. The public tree is published for
source transparency and local builds under the terms of the Apache License,
Version 2.0 (see [`LICENSE`](LICENSE)).

## Security reports

Please report security issues through the process described in
[`SECURITY.md`](SECURITY.md). Do not open a public issue for vulnerabilities.

## Local builds

A bare shell (invoking `cargo` directly instead of via `build.ps1` /
`build.bat`) must set `PROTOC=D:\CLI\orz\bin\protoc.exe` — the build script
of the `orz-tools-api` codegen crate depends on it (verified failing without
it on 2026-10-16).

## Licensing of this source

By downloading or using this source, you agree that your use is governed by
the Apache License, Version 2.0. No contributor license agreement is offered
because external contributions are not accepted.
