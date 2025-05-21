@echo off
:: ============================================================
::         Jellyfin Rename - Uninstaller Script
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
echo   Uninstalling Jellyfin Rename...
echo ============================================================
echo.

:: Remove registry entries
echo [1/2] Removing context menu entry from registry...
reg delete "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" /f

:: Remove installation directory
echo [2/2] Removing installation files...
powershell -Command "Remove-Item -Path 'C:\Program Files\JellyfinRename' -Force -Recurse -ErrorAction SilentlyContinue"

if exist "C:\Program Files\JellyfinRename" (
    echo.
    echo Warning: Some files could not be removed. You may need to delete them manually.
) else (
    echo.
    echo All files successfully removed.
)

echo.
echo ============================================================
echo   Uninstallation complete!
echo ============================================================
echo.
pause
