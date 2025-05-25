@echo off
:: ============================================================
::           Jellyfin Rename - Installer Script
:: ============================================================

:: Enable color support and strict error handling
setlocal EnableDelayedExpansion
set "INSTALL_DIR=C:\Program Files\JellyfinRename"
set "INSTALLED_FILES=false"
set "REGISTRY_UPDATED=false"

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

:: Pre-installation checks
echo [0/4] Performing pre-installation checks...

:: Check if files exist before starting
if not exist "%~dp0jellyfin-rename.exe" (
    echo.
    powershell -Command "Write-Host '[ERROR] Could not find jellyfin-rename.exe in current directory!' -ForegroundColor Red"
    powershell -Command "Write-Host 'Expected location: %~dp0jellyfin-rename.exe' -ForegroundColor Yellow"
    goto :error
)

if not exist "%~dp0assets\jellyfin.ico" (
    echo.
    powershell -Command "Write-Host '[ERROR] Could not find jellyfin.ico!' -ForegroundColor Red"
    powershell -Command "Write-Host 'Expected location: %~dp0assets\jellyfin.ico' -ForegroundColor Yellow"
    goto :error
)

if not exist "%~dp0registry\menu-option.reg" (
    echo.
    powershell -Command "Write-Host '[ERROR] Could not find menu-option.reg!' -ForegroundColor Red"
    powershell -Command "Write-Host 'Expected location: %~dp0registry\menu-option.reg' -ForegroundColor Yellow"
    goto :error
)

:: Check if already installed
if exist "%INSTALL_DIR%" (
    echo.
    powershell -Command "Write-Host '[WARNING] Jellyfin Rename appears to already be installed.' -ForegroundColor Yellow"
    set /p "CONTINUE=Do you want to reinstall? (y/N): "
    if /i "!CONTINUE!" neq "y" (
        echo Installation cancelled by user.
        goto :end
    )
    echo.
    powershell -Command "Write-Host 'Removing existing installation...' -ForegroundColor Yellow"
    powershell -Command "Remove-Item -Path '%INSTALL_DIR%' -Force -Recurse -ErrorAction SilentlyContinue"
)

echo All pre-installation checks passed.
echo.

:: Create installation directory
echo [1/4] Creating installation directory...
mkdir "%INSTALL_DIR%" 2>nul
if %ERRORLEVEL% neq 0 (
    echo.
    powershell -Command "Write-Host '[ERROR] Failed to create installation directory!' -ForegroundColor Red"
    powershell -Command "Write-Host 'This may be due to insufficient permissions or disk space.' -ForegroundColor Yellow"
    goto :error
)

:: Copy the executable
echo [2/4] Copying executable...
copy /Y "%~dp0jellyfin-rename.exe" "%INSTALL_DIR%\" >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo.
    powershell -Command "Write-Host '[ERROR] Failed to copy executable!' -ForegroundColor Red"
    goto :error
)
set "INSTALLED_FILES=true"

echo [3/4] Copying icon...
copy /Y "%~dp0assets\jellyfin.ico" "%INSTALL_DIR%\" >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo.
    powershell -Command "Write-Host '[ERROR] Failed to copy icon file!' -ForegroundColor Red"
    goto :error
)

:: Import registry entries
echo [4/4] Adding context menu entry to registry...
regedit /s "%~dp0registry\menu-option.reg" 2>nul
if %ERRORLEVEL% neq 0 (
    echo.
    powershell -Command "Write-Host '[ERROR] Failed to import registry entries!' -ForegroundColor Red"
    powershell -Command "Write-Host 'The registry modification may have been blocked by antivirus software.' -ForegroundColor Yellow"
    goto :error
)
set "REGISTRY_UPDATED=true"

echo.
echo ============================================================
echo   Installation complete!
echo ============================================================
echo   Right-click on any file and select "Jellyfin Rename"
echo   to use the tool.
echo.
echo   Installation location: %INSTALL_DIR%
echo   To uninstall, run the uninstall.bat script.
echo.
goto :end

:error
echo.
powershell -Command "Write-Host '============================================================' -ForegroundColor Red"
powershell -Command "Write-Host '  Installation failed! Cleaning up...' -ForegroundColor Red"
powershell -Command "Write-Host '============================================================' -ForegroundColor Red"

:: Cleanup on error
if "%INSTALLED_FILES%"=="true" (
    echo.
    echo Removing partially installed files...
    if exist "%INSTALL_DIR%" (
        powershell -Command "Remove-Item -Path '%INSTALL_DIR%' -Force -Recurse -ErrorAction SilentlyContinue"
        if exist "%INSTALL_DIR%" (
            powershell -Command "Write-Host '[WARNING] Could not fully clean up installation directory.' -ForegroundColor Yellow"
            powershell -Command "Write-Host 'You may need to manually delete: %INSTALL_DIR%' -ForegroundColor Yellow"
        ) else (
            echo Cleanup successful.
        )
    )
)

if "%REGISTRY_UPDATED%"=="true" (
    echo.
    echo Removing registry entries...
    reg delete "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" /f 2>nul >nul
)

echo.
powershell -Command "Write-Host 'Installation failed! Please check the errors above and try again.' -ForegroundColor Red"
echo.

:end
pause
endlocal
