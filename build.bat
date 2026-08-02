@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if errorlevel 1 (
    echo vcvars64.bat failed
    exit /b 1
)
set PATH=B:\.cargo\bin;%PATH%
set CARGO_HOME=B:\.cargo
set RUSTUP_HOME=B:\.rustup
cd /d B:\orz
where link.exe
cargo check 2>&1
