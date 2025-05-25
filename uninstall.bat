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
echo   Jellyfin Rename - Uninstaller
echo ============================================================
echo.

:: Check if anything is installed
echo [0/3] Checking installation status...

set "SOMETHING_TO_REMOVE=false"

:: Check registry entries
reg query "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    set "SOMETHING_TO_REMOVE=true"
    powershell -Command "Write-Host '  - Context menu entry found in registry' -ForegroundColor Yellow"
)

:: Check installation directory (including empty directories)
if exist "%INSTALL_DIR%" (
    set "SOMETHING_TO_REMOVE=true"
    powershell -Command "Write-Host '  - Installation directory found: %INSTALL_DIR%' -ForegroundColor Yellow"
    
    :: Check if directory has any files
    dir /b "%INSTALL_DIR%" >nul 2>&1
    if %ERRORLEVEL% equ 0 (
        powershell -Command "Write-Host '    (Directory contains files)' -ForegroundColor Cyan"
    ) else (
        powershell -Command "Write-Host '    (Directory is empty - likely from failed installation)' -ForegroundColor Magenta"
    )
)

if "%SOMETHING_TO_REMOVE%"=="false" (
    echo.
    powershell -Command "Write-Host '[INFO] Jellyfin Rename does not appear to be installed.' -ForegroundColor Green"
    powershell -Command "Write-Host 'Nothing to uninstall.' -ForegroundColor Green"
    goto :end
)

:: Confirmation prompt
echo.
powershell -Command "Write-Host 'The following will be removed:' -ForegroundColor Yellow"
reg query "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    powershell -Command "Write-Host '  - Context menu registry entries' -ForegroundColor White"
)
if exist "%INSTALL_DIR%" (
    powershell -Command "Write-Host '  - Installation directory and all contents' -ForegroundColor White"
)
echo.
set /p "CONFIRM=Are you sure you want to continue? (y/N): "
if /i "!CONFIRM!" neq "y" (
    echo.
    powershell -Command "Write-Host 'Uninstallation cancelled by user.' -ForegroundColor Cyan"
    goto :end
)

echo.
echo ============================================================
echo   Uninstalling Jellyfin Rename...
echo ============================================================
echo.

:: Remove registry entries
echo [1/3] Removing context menu entry from registry...
reg query "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    reg delete "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" /f >nul 2>&1
    if %ERRORLEVEL% equ 0 (
        set "REGISTRY_REMOVED=true"
        powershell -Command "Write-Host '  Registry entries successfully removed.' -ForegroundColor Green"
    ) else (
        powershell -Command "Write-Host '[WARNING] Failed to remove registry entries!' -ForegroundColor Red"
        powershell -Command "Write-Host 'You may need to remove them manually using regedit.' -ForegroundColor Yellow"
    )
) else (
    powershell -Command "Write-Host '  No registry entries found to remove.' -ForegroundColor Gray"
    set "REGISTRY_REMOVED=true"
)

:: Remove installation directory
echo [2/3] Removing installation files...
if not exist "%INSTALL_DIR%" (
    powershell -Command "Write-Host '  No installation directory found to remove.' -ForegroundColor Gray"
    set "FILES_REMOVED=true"
) else (
    :: Try to remove the directory
    powershell -Command "Remove-Item -Path '%INSTALL_DIR%' -Force -Recurse -ErrorAction SilentlyContinue"
    
    :: Check if removal was successful
    if exist "%INSTALL_DIR%" (
        :: Check if directory is now empty
        dir /b "%INSTALL_DIR%" >nul 2>&1
        if %ERRORLEVEL% neq 0 (
            :: Directory is empty, try to remove it
            rmdir "%INSTALL_DIR%" >nul 2>&1
            if not exist "%INSTALL_DIR%" (
                set "FILES_REMOVED=true"
                powershell -Command "Write-Host '  Empty installation directory successfully removed.' -ForegroundColor Green"
            ) else (
                powershell -Command "Write-Host '[WARNING] Could not remove empty installation directory.' -ForegroundColor Yellow"
                powershell -Command "Write-Host 'Path: %INSTALL_DIR%' -ForegroundColor Yellow"
            )
        ) else (
            powershell -Command "Write-Host '[WARNING] Some files could not be removed!' -ForegroundColor Red"
            powershell -Command "Write-Host 'This may be because:' -ForegroundColor Yellow"
            powershell -Command "Write-Host '  - Files are currently in use' -ForegroundColor Yellow"
            powershell -Command "Write-Host '  - Insufficient permissions' -ForegroundColor Yellow"
            powershell -Command "Write-Host '  - Files are protected by antivirus' -ForegroundColor Yellow"
            powershell -Command "Write-Host 'Remaining files in: %INSTALL_DIR%' -ForegroundColor Yellow"
            
            :: List remaining files
            echo.
            powershell -Command "Write-Host 'Remaining files:' -ForegroundColor Yellow"
            dir "%INSTALL_DIR%" /b 2>nul | powershell -Command "$input | ForEach-Object { Write-Host \"  - $_\" -ForegroundColor White }"
        )
    ) else (
        set "FILES_REMOVED=true"
        powershell -Command "Write-Host '  All installation files successfully removed.' -ForegroundColor Green"
    )
)

:: Final verification and summary
echo [3/3] Verifying uninstallation...

set "UNINSTALL_SUCCESS=true"

:: Check registry
reg query "HKEY_CLASSES_ROOT\*\shell\JellyfinRename" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    set "UNINSTALL_SUCCESS=false"
    powershell -Command "Write-Host '  [!] Registry entries still present' -ForegroundColor Red"
) else (
    powershell -Command "Write-Host '  Registry entries: Removed' -ForegroundColor Green"
)

:: Check files
if exist "%INSTALL_DIR%" (
    dir /b "%INSTALL_DIR%" >nul 2>&1
    if %ERRORLEVEL% equ 0 (
        set "UNINSTALL_SUCCESS=false"
        powershell -Command "Write-Host '  [!] Installation files still present' -ForegroundColor Red"
    ) else (
        powershell -Command "Write-Host '  [!] Empty installation directory still present' -ForegroundColor Yellow"
    )
) else (
    powershell -Command "Write-Host '  Installation files: Removed' -ForegroundColor Green"
)

echo.
if "%UNINSTALL_SUCCESS%"=="true" (
    powershell -Command "Write-Host '============================================================' -ForegroundColor Green"
    powershell -Command "Write-Host '  Uninstallation completed successfully!' -ForegroundColor Green"
    powershell -Command "Write-Host '============================================================' -ForegroundColor Green"
    powershell -Command "Write-Host '  Jellyfin Rename has been completely removed.' -ForegroundColor Green"
) else (
    powershell -Command "Write-Host '============================================================' -ForegroundColor Yellow"
    powershell -Command "Write-Host '  Uninstallation completed with warnings!' -ForegroundColor Yellow"
    powershell -Command "Write-Host '============================================================' -ForegroundColor Yellow"
    powershell -Command "Write-Host '  Some components could not be automatically removed.' -ForegroundColor Yellow"
    powershell -Command "Write-Host '  Please check the warnings above for manual cleanup steps.' -ForegroundColor Yellow"
)

:end
echo.
pause
endlocal
