@echo off
rem Builds OVRLEY and creates the signed Windows installer with Tauri (NSIS), then prepares the GitHub release files.
rem   build-installer.bat        NSIS installer (.exe) + update-site\ folder with the release files
rem   build-installer.bat msi    MSI installer (needs WiX), no update-site folder
rem The installer ends up in src-tauri\target\release\bundle\<type>\.
rem Optional: set OVRLEY_NOTES=text before running to put release notes in latest.json.

setlocal
cd /d "%~dp0"

rem pnpm installed through corepack and cargo live here on this machine.
if exist "%USERPROFILE%\.corepack-bin" set "PATH=%USERPROFILE%\.corepack-bin;%PATH%"
if exist "%USERPROFILE%\.cargo\bin" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

rem tauri.conf.json asks for signed auto-update artifacts: sign them with the private key kept outside the repository.
rem Keep a backup of this key; without it installed copies can no longer be updated.
rem The key password is read from a file next to the key (never stored in the repository).
set "TAURI_SIGNING_PRIVATE_KEY=%USERPROFILE%\.tauri\ovrley-luizleite.key"
set "TAURI_SIGNING_PASSWORD_FILE=%USERPROFILE%\.tauri\ovrley-luizleite.password"
if not exist "%TAURI_SIGNING_PRIVATE_KEY%" (
  echo Signing key not found: %TAURI_SIGNING_PRIVATE_KEY%
  pause
  exit /b 1
)
if not exist "%TAURI_SIGNING_PASSWORD_FILE%" (
  echo Signing key password file not found: %TAURI_SIGNING_PASSWORD_FILE%
  pause
  exit /b 1
)
set /p TAURI_SIGNING_PRIVATE_KEY_PASSWORD=<"%TAURI_SIGNING_PASSWORD_FILE%"

set "BUNDLE=nsis"
if /i "%~1"=="msi" set "BUNDLE=msi"

echo.
echo === Building the OVRLEY %BUNDLE% installer (pnpm tauri build --bundles %BUNDLE%) ===
call pnpm tauri build --bundles %BUNDLE%
if errorlevel 1 (
  echo.
  echo The build failed. No installer was created.
  pause
  exit /b 1
)

if /i "%BUNDLE%"=="nsis" (
  echo.
  echo === Preparing the GitHub release files ===
  if defined OVRLEY_NOTES (
    node scripts\prepare-update-site.mjs --notes "%OVRLEY_NOTES%"
  ) else (
    node scripts\prepare-update-site.mjs
  )
  if errorlevel 1 (
    echo.
    echo Could not prepare the release files.
    pause
    exit /b 1
  )
  explorer "%~dp0update-site"
) else (
  explorer "%~dp0src-tauri\target\release\bundle\%BUNDLE%"
)

echo.
echo Done.
pause
exit /b 0
