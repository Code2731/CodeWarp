@echo off
setlocal EnableExtensions

set "ROOT_DIR=%~dp0.."
pushd "%ROOT_DIR%" >nul
if errorlevel 1 (
    echo [run] Failed to enter project root: "%ROOT_DIR%"
    exit /b 1
)

where cargo >nul 2>nul
if errorlevel 1 (
    echo [run] cargo is not installed or not in PATH.
    popd
    exit /b 1
)

echo [run] Launching CodeWarp with cargo run...
cargo run -- %*
set "EXIT_CODE=%ERRORLEVEL%"

popd
exit /b %EXIT_CODE%
