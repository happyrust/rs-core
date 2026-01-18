@echo off
REM FITT ORI/POS Hook 启动脚本
REM 用于附加到已运行的 AVEVA E3D 进程

echo ============================================================
echo   FITT ORI/POS Hook for AVEVA E3D
echo ============================================================
echo.

REM 检查 Python 是否可用
python --version >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Python not found. Please install Python 3.8+
    pause
    exit /b 1
)

REM 检查 Frida 是否已安装
python -c "import frida" >nul 2>&1
if errorlevel 1 (
    echo [INFO] Installing Frida...
    pip install frida frida-tools
)

REM 获取脚本目录
set SCRIPT_DIR=%~dp0

REM 运行 hook
echo [*] Starting FITT hook...
echo [*] Press Ctrl+C to stop
echo.

python "%SCRIPT_DIR%run_fitt_hook.py" --attach --verbose

echo.
echo [*] Hook stopped
pause
