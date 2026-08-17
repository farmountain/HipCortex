@echo off
:: HipCortex MCP launcher for Claude Code
:: Starts Rust server if not running, then serves MCP over stdio

set HIPCORTEX_DIR=D:\all_projects\hipcortex
set HIPCORTEX_URL=http://localhost:3030
set PYTHON=C:\Users\user\.openharness-venv\Scripts\python.exe

:: Start Rust server if port 3030 not listening
netstat -an 2>nul | findstr ":3030 " | findstr LISTENING >nul 2>&1
if errorlevel 1 (
    start /B "" "%HIPCORTEX_DIR%\target\release\webserver.exe" > "%TEMP%\hipcortex-server.log" 2>&1
    timeout /t 2 /nobreak > nul
)

:: Run MCP server over stdio
"%PYTHON%" "%HIPCORTEX_DIR%\sdk\mcp\server.py"
