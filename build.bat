@echo off
setlocal
set RUSTPATH=%USERPROFILE%\.cargo\bin
set PATH=C:\Windows\system32;C:\Windows;C:\Windows\System32\Wbem;%RUSTPATH%
set CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld
cd /d E:\geo\geo-toolbox

if "%1"=="" goto help
if "%1"=="check"  goto check
if "%1"=="build"  goto build
if "%1"=="test"   goto test
if "%1"=="run"    goto run
if "%1"=="clean"  goto clean
goto help

:help
echo geo-toolbox build helper
echo.
echo   build check    cargo check (fast, no binary)
echo   build build    cargo build (release binary)
echo   build test     cargo test -p geo-core
echo   build run ARGS cargo run -- ARGS
echo   build clean    cargo clean
goto :eof

:check
cargo check
goto :eof

:build
cargo build --release
goto :eof

:test
cargo test -p geo-core %2 %3 %4 %5
goto :eof

:run
shift
cargo run -- %1 %2 %3 %4 %5 %6 %7 %8 %9
goto :eof

:clean
cargo clean
goto :eof
