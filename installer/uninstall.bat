@echo off
:: ============================================================
::         Jellyfin Rename - Uninstaller Script
:: ============================================================

:: Enable color support and strict error handling
setlocal EnableDelayedExpansion
set "INSTALL_DIR=C:\Program Files\JellyfinRename"
set "REGISTRY_KEY=HKEY_CLASSES_ROOT\*\shell\JellyfinRename"
set "ERRORS_OCCURRED=false"

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
echo [0/2] Checking installation status...

:: Check if software is actually installed
set "SOFTWARE_FOUND=false"
if exist "%INSTALL_DIR%" (
    set "SOFTWARE_FOUND=true"
    echo Installation directory found: %INSTALL_DIR%
)

reg query "%REGISTRY_KEY%" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    set "SOFTWARE_FOUND=true"
    echo Registry entries found.
)

if "%SOFTWARE_FOUND%"=="false" (
    echo.
    powershell -Command "Write-Host '[INFO] Jellyfin Rename does not appear to be installed.' -ForegroundColor Cyan"
    powershell -Command "Write-Host 'No installation directory or registry entries found.' -ForegroundColor Cyan"
    set /p "CONTINUE=Continue anyway to clean up any remaining files? (y/N): "
    if /i "!CONTINUE!" neq "y" (
        echo Uninstallation cancelled by user.
        goto :end
    )
    echo.
) else (
    echo Software installation detected.
    echo.
    set /p "CONFIRM=Are you sure you want to uninstall Jellyfin Rename? (y/N): "
    if /i "!CONFIRM!" neq "y" (
        echo Uninstallation cancelled by user.
        goto :end
    )
    echo.
)

:: Remove registry entries
echo [1/2] Removing context menu entry from registry...
reg query "%REGISTRY_KEY%" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    reg delete "%REGISTRY_KEY%" /f >nul 2>&1
    if %ERRORLEVEL% equ 0 (
        echo Registry entries successfully removed.
    ) else (
        echo.
        powershell -Command "Write-Host '[ERROR] Failed to remove registry entries!' -ForegroundColor Red"
        powershell -Command "Write-Host 'This may be due to antivirus software blocking the operation.' -ForegroundColor Yellow"
        set "ERRORS_OCCURRED=true"
    )
) else (
    echo Registry entries not found (already removed or never installed).
)

:: Remove installation directory
echo [2/2] Removing installation files...
if not exist "%INSTALL_DIR%" (
    echo Installation directory not found (already removed or never installed).
) else (
    echo Removing files from: %INSTALL_DIR%
    
    :: Try to terminate any running processes first
    tasklist /FI "IMAGENAME eq jellyfin-rename.exe" 2>nul | find /I "jellyfin-rename.exe" >nul
    if %ERRORLEVEL% equ 0 (
        echo.
        powershell -Command "Write-Host '[WARNING] Jellyfin Rename appears to be running.' -ForegroundColor Yellow"
        echo Attempting to close the application...
        taskkill /F /IM "jellyfin-rename.exe" >nul 2>&1
        timeout /t 2 >nul
    )
    
    :: Remove the directory
    powershell -Command "Remove-Item -Path '%INSTALL_DIR%' -Force -Recurse -ErrorAction SilentlyContinue"
    
    :: Verify removal
    if exist "%INSTALL_DIR%" (
        echo.
        powershell -Command "Write-Host '[ERROR] Some files could not be removed!' -ForegroundColor Red"
        powershell -Command "Write-Host 'This may be because:' -ForegroundColor Yellow"
        powershell -Command "Write-Host '  - Files are currently in use' -ForegroundColor Yellow"
        powershell -Command "Write-Host '  - Insufficient permissions' -ForegroundColor Yellow"
        powershell -Command "Write-Host '  - Files are locked by another process' -ForegroundColor Yellow"
        echo.
        powershell -Command "Write-Host 'Manual removal may be required: %INSTALL_DIR%' -ForegroundColor Yellow"
        set "ERRORS_OCCURRED=true"
    ) else (
        echo All files successfully removed.
    )
)

echo.
if "%ERRORS_OCCURRED%"=="true" (
    echo ============================================================
    powershell -Command "Write-Host '  Uninstallation completed with errors!' -ForegroundColor Yellow"
    echo ============================================================
    echo   Some components could not be removed automatically.
    echo   Please check the messages above for manual cleanup steps.
) else (
    echo ============================================================
    powershell -Command "Write-Host '  Uninstallation complete!' -ForegroundColor Green"
    echo ============================================================
    echo   Jellyfin Rename has been successfully removed from your system.
    echo   Thank you for using Jellyfin Rename!
)
echo.

:end
pause
endlocal
