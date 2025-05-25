@echo off
:: ============================================================
::         Jellyfin Rename - Uninstaller Script
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
echo   Uninstalling Jellyfin Rename...
echo ============================================================
echo.

:: Remove registry entries
echo [1/2] Removing context menu entry from registry...
reg delete "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" /f 2>nul
if %ERRORLEVEL% neq 0 (
    echo.
    powershell -Command "Write-Host '[ERROR] Failed to remove registry entries!' -ForegroundColor Red"
)

:: Remove installation directory
echo [2/2] Removing installation files...
if not exist "C:\Program Files\JellyfinRename" (
    echo.
    powershell -Command "Write-Host '[WARNING] Installation directory not found. Nothing to remove.' -ForegroundColor Yellow"
) else (
    powershell -Command "Remove-Item -Path 'C:\Program Files\JellyfinRename' -Force -Recurse -ErrorAction SilentlyContinue"
    
    if exist "C:\Program Files\JellyfinRename" (
        echo.
        powershell -Command "Write-Host '[ERROR] Some files could not be removed. You may need to delete them manually.' -ForegroundColor Red"
    ) else (
        echo.
        echo All files successfully removed.
    )
)

echo.
echo ============================================================
echo   Uninstallation complete!
echo ============================================================
echo.
pause
endlocal
