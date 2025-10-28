@echo off
echo ====================================
echo Building Esponquen Distribution
echo ====================================
echo.

REM Build in release mode
echo [1/5] Building release binary...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Build failed!
    exit /b %ERRORLEVEL%
)
echo ✓ Build successful
echo.

REM Create dist directory
echo [2/5] Creating dist directory...
if exist dist rmdir /s /q dist
mkdir dist
echo ✓ Created dist directory
echo.

REM Copy the binary
echo [3/5] Copying binary...
copy target\release\esponquen.exe dist\
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Failed to copy binary!
    exit /b %ERRORLEVEL%
)
echo ✓ Binary copied
echo.

REM Copy model directory
echo [4/5] Copying model directory...
if exist model (
    xcopy model dist\model\ /E /I /Y
    if %ERRORLEVEL% NEQ 0 (
        echo ERROR: Failed to copy model directory!
        exit /b %ERRORLEVEL%
    )
    echo ✓ Model directory copied
) else (
    echo WARNING: model directory not found, skipping...
)
echo.

REM Copy icons directory
echo [5/5] Copying icons directory...
if exist icons (
    xcopy icons dist\icons\ /E /I /Y
    if %ERRORLEVEL% NEQ 0 (
        echo ERROR: Failed to copy icons directory!
        exit /b %ERRORLEVEL%
    )
    echo ✓ Icons directory copied
) else (
    echo WARNING: icons directory not found, skipping...
)
echo.

REM Copy any DLL files from target\release
echo [OPTIONAL] Copying DLL files...
if exist target\release\*.dll (
    copy target\release\*.dll dist\
    echo ✓ DLL files copied
) else (
    echo No DLL files found in target\release
)
echo.

echo ====================================
echo Distribution package created successfully!
echo ====================================
echo.
echo Location: dist\esponquen.exe
echo.
echo Contents:
dir /B dist
echo.
echo Run: dist\esponquen.exe
echo Or with console: dist\esponquen.exe --console
echo.
pause
