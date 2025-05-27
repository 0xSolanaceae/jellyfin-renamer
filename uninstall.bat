@echo off
:: ============================================================
::         Jellyfin Rename - Uninstaller Script
:: ============================================================

:: Enable color support and strict error handling
setlocal EnableDelayedExpansion
set "INSTALL_DIR=C:\Program Files\JellyfinRename"
set "REGISTRY_REMOVED=false"
set "FILES_REMOVED=false"

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

:: Pre-uninstallation checks
echo [0/3] Performing pre-uninstallation checks...

:: Check if anything is installed
set "SOMETHING_TO_REMOVE=false"

reg query "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    set "SOMETHING_TO_REMOVE=true"
)

if exist "%INSTALL_DIR%" (
    set "SOMETHING_TO_REMOVE=true"
)

if "%SOMETHING_TO_REMOVE%"=="false" (
    echo.
    powershell -Command "Write-Host '[INFO] Jellyfin Rename is not installed.' -ForegroundColor Green"
    echo Nothing to uninstall.
    goto :end
)

:: Confirmation prompt
echo.
powershell -Command "Write-Host '[WARNING] This will remove Jellyfin Rename completely.' -ForegroundColor Yellow"
set /p "CONFIRM=Do you want to continue? (y/N): "
if /i "!CONFIRM!" neq "y" (
    echo Uninstallation cancelled by user.
    goto :end
)

echo.
echo All pre-uninstallation checks passed.
echo.

:: Remove registry entries
echo [1/3] Removing context menu entry from registry...
reg query "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    reg delete "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" /f >nul 2>&1
    if %ERRORLEVEL% neq 0 (
        echo.
        powershell -Command "Write-Host '[ERROR] Failed to remove registry entries!' -ForegroundColor Red"
        goto :error
    )
)
set "REGISTRY_REMOVED=true"

:: Remove installation directory
echo [2/3] Removing installation files...
if exist "%INSTALL_DIR%" (
    powershell -Command "Remove-Item -Path '%INSTALL_DIR%' -Force -Recurse -ErrorAction SilentlyContinue"
    if exist "%INSTALL_DIR%" (
        echo.
        powershell -Command "Write-Host '[ERROR] Failed to remove installation directory!' -ForegroundColor Red"
        powershell -Command "Write-Host 'Files may be in use or protected by antivirus software.' -ForegroundColor Yellow"
        goto :error
    )
)
set "FILES_REMOVED=true"

:: Final verification
echo [3/3] Verifying removal...
reg query "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    echo.
    powershell -Command "Write-Host '[ERROR] Registry entries still present!' -ForegroundColor Red"
    goto :error
)

if exist "%INSTALL_DIR%" (
    echo.
    powershell -Command "Write-Host '[ERROR] Installation directory still exists!' -ForegroundColor Red"
    goto :error
)

echo.
echo ============================================================
echo   Uninstallation complete!
echo ============================================================
echo   Jellyfin Rename has been completely removed from
echo   your system.
echo.
goto :end

:error
echo.
powershell -Command "Write-Host '============================================================' -ForegroundColor Red"
powershell -Command "Write-Host '  Uninstallation failed! Attempting cleanup...' -ForegroundColor Red"
powershell -Command "Write-Host '============================================================' -ForegroundColor Red"

:: Cleanup on error - try to remove what we can
if "%REGISTRY_REMOVED%"=="false" (
    echo.
    echo Attempting to remove registry entries...
    reg delete "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" /f 2>nul >nul
)

if "%FILES_REMOVED%"=="false" (
    echo.
    echo Attempting to remove installation files...
    if exist "%INSTALL_DIR%" (
        powershell -Command "Remove-Item -Path '%INSTALL_DIR%' -Force -Recurse -ErrorAction SilentlyContinue"
        if exist "%INSTALL_DIR%" (
            powershell -Command "Write-Host '[WARNING] Could not fully remove installation directory.' -ForegroundColor Yellow"
            powershell -Command "Write-Host 'You may need to manually delete: %INSTALL_DIR%' -ForegroundColor Yellow"
        )
    )
)

echo.
powershell -Command "Write-Host 'Uninstallation failed! Please check the errors above and try again.' -ForegroundColor Red"
echo.

:end
pause
endlocal
