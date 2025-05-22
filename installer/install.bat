@echo off
:: ============================================================
::           Jellyfin Rename - Installer Script
:: ============================================================

:: Enable color support
setlocal EnableDelayedExpansion

:: Check for administrative privileges
NET SESSION >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo.
    echo ============================================================
    echo   Administrator privileges required. Requesting elevation...
    echo ============================================================
    powershell -Command "Start-Process -FilePath '%~dpnx0' -Verb RunAs"
    exit /b
)

echo.
echo ============================================================
echo   Installing Jellyfin Rename...
echo ============================================================
echo.

:: Create installation directory
echo [1/4] Creating installation directory...
mkdir "C:\Program Files\JellyfinRename" 2>nul
if %ERRORLEVEL% neq 0 (
    echo.
    powershell -Command "Write-Host '[ERROR] Failed to create installation directory!' -ForegroundColor Red"
    goto :error
)

:: Copy the executable and icon
echo [2/4] Copying executable...
if not exist "%~dp0..\jellyfin-rename.exe" (
    echo.
    powershell -Command "Write-Host '[ERROR] Could not find jellyfin-rename.exe!' -ForegroundColor Red"
    goto :error
)
copy /Y "%~dp0..\jellyfin-rename.exe" "C:\Program Files\JellyfinRename\" >nul
if %ERRORLEVEL% neq 0 (
    echo.
    powershell -Command "Write-Host '[ERROR] Failed to copy executable!' -ForegroundColor Red"
    goto :error
)

echo [3/4] Copying icon...
if not exist "%~dp0..\assets\jellyfin.ico" (
    echo.
    powershell -Command "Write-Host '[ERROR] Could not find jellyfin.ico!' -ForegroundColor Red"
    goto :error
)
copy /Y "%~dp0..\assets\jellyfin.ico" "C:\Program Files\JellyfinRename\" >nul
if %ERRORLEVEL% neq 0 (
    echo.
    powershell -Command "Write-Host '[ERROR] Failed to copy icon file!' -ForegroundColor Red"
    goto :error
)

:: Import registry entries
echo [4/4] Adding context menu entry to registry...
if not exist "%~dp0menu-option.reg" (
    echo.
    powershell -Command "Write-Host '[ERROR] Could not find menu-option.reg!' -ForegroundColor Red"
    goto :error
)
regedit /s "%~dp0menu-option.reg"
if %ERRORLEVEL% neq 0 (
    echo.
    powershell -Command "Write-Host '[ERROR] Failed to import registry entries!' -ForegroundColor Red"
    goto :error
)

echo.
echo ============================================================
echo   Installation complete!
echo ============================================================
echo   Right-click on any file and select "Jellyfin Rename"
echo   to use the tool.
echo.
goto :end

:error
echo.
echo.
powershell -Command "Write-Host 'Installation failed! Please check the errors above.' -ForegroundColor Red"
echo.

:end
pause
endlocal
