@echo off
REM run_match.bat — Automated engine match pipeline for Project Odin
REM
REM Builds the current engine, ensures a baseline binary exists,
REM runs the match manager, and reports Elo + SPRT results.
REM
REM Usage: run_match.bat

setlocal

set SCRIPT_DIR=%~dp0
set PROJECT_DIR=%SCRIPT_DIR%..
set ENGINE=%PROJECT_DIR%\target\release\odin-engine.exe
set BASELINE=%PROJECT_DIR%\target\release\odin-engine-baseline.exe

echo ============================================================
echo  Project Odin — Engine Match Pipeline
echo ============================================================
echo.

REM Step 1: Build the current engine
echo [1/3] Building current engine...
cd /d "%PROJECT_DIR%"
cargo build --release
if errorlevel 1 (
    echo ERROR: Build failed.
    exit /b 1
)
echo Build complete.
echo.

REM Step 2: Ensure baseline binary exists
if not exist "%BASELINE%" (
    echo [2/3] No baseline binary found. Creating baseline from current build...
    copy "%ENGINE%" "%BASELINE%" >nul
    echo Baseline created: %BASELINE%
    echo NOTE: First run — matching engine against itself. Elo should be ~0.
) else (
    echo [2/3] Baseline binary found: %BASELINE%
)
echo.

REM Step 3: Run the match
echo [3/3] Running match...
echo.
cd /d "%SCRIPT_DIR%"
node match.mjs
if errorlevel 1 (
    echo ERROR: Match failed.
    exit /b 1
)

echo.
echo ============================================================
echo  Match complete. Results in observer\match_reports\
echo ============================================================
echo.

REM Offer baseline promotion
set /p PROMOTE="Promote current engine as new baseline? (y/N): "
if /i "%PROMOTE%"=="y" (
    copy "%ENGINE%" "%BASELINE%" >nul
    echo Baseline updated.
) else (
    echo Baseline unchanged.
)
