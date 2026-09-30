@echo off
rem Double-click launcher for Windows (ADR 0117): runs tools/launch.sh in Git Bash, which
rem rebuilds and reimports what changed, then starts the game.
setlocal
cd /d "%~dp0"
set "GITBASH="
if exist "%ProgramFiles%\Git\bin\bash.exe" set "GITBASH=%ProgramFiles%\Git\bin\bash.exe"
if not defined GITBASH if exist "%LocalAppData%\Programs\Git\bin\bash.exe" set "GITBASH=%LocalAppData%\Programs\Git\bin\bash.exe"
if not defined GITBASH for /f "delims=" %%G in ('where git.exe 2^>nul') do if not defined GITBASH if exist "%%~dpG..\bin\bash.exe" set "GITBASH=%%~dpG..\bin\bash.exe"
if not defined GITBASH (
  echo [Cent Ans] Git pour Windows ^(Git Bash^) est necessaire : https://git-scm.com/download/win
  pause
  exit /b 1
)
"%GITBASH%" tools/launch.sh %*
set "RC=%errorlevel%"
rem No pause in CI (GitHub Actions sets CI): nobody reads the window there.
if not "%RC%"=="0" if not defined CI pause
exit /b %RC%
