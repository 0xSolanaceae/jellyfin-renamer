@echo off
:: ============================================================
::           Jellyfin Rename - Installer Script
:: ============================================================

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

:: Copy the executable and icon
echo [2/4] Copying executable...
copy /Y "%~dp0..\target\release\jellyfin-rename.exe" "C:\Program Files\JellyfinRename\"
echo [3/4] Copying icon...
copy /Y "%~dp0..\assets\jellyfin.ico" "C:\Program Files\JellyfinRename\"

:: Import registry entries
echo [4/4] Adding context menu entry to registry...
regedit /s "%~dp0menu-option.reg"

echo.
echo ============================================================
echo   Installation complete!
echo ============================================================
echo   Right-click on any file and select "Jellyfin Rename"
echo   to use the tool.
echo.
pause
