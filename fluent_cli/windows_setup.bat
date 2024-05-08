@echo off
setlocal

:: User's home directory variable
set "USER_HOME=c:\users\%USERNAME%"

:: Path to the directory where the tarball will be extracted
set "EXTRACT_DIR=%USER_HOME%\.fluent_cli"

:: Extract the tarball
echo Extracting files...
tar -zxvf .\fluent-x86_64-pc-windows-msvc-v.0.3.5.1.tar.gz -C "%EXTRACT_DIR%"

:: Move the files up one directory level
echo Moving files...
move "%EXTRACT_DIR%\fluent_cli\*" "%EXTRACT_DIR%\"

:: Remove the now empty directory
echo Cleaning up...
rmdir "%EXTRACT_DIR%\fluent_cli"

:: Set environment variables
echo Setting environment variables...
setx FLUENT_CLI_CONFIG_PATH "%EXTRACT_DIR%\config.json"
setx AMBER_YAML "%EXTRACT_DIR%\amber.yaml"

:: Add the extraction directory to the system PATH
set "NEW_PATH=%EXTRACT_DIR%;%PATH%"
setx PATH "%NEW_PATH%"

:: Ask the user for AMBER_SECRET or if they need to init
echo Please choose an option:
echo 1. Enter AMBER_SECRET
echo 2. Initialize Amber (amber init)
set /p user_option="Enter your choice (1 or 2): "

if "%user_option%"=="1" (
    set /p AMBER_SECRET="Enter your AMBER_SECRET: "
    setx AMBER_SECRET "%AMBER_SECRET%"
) else if "%user_option%"=="2" (
    cd "%EXTRACT_DIR%"
    echo Initializing Amber...
    amber init
    echo Copy that key and now enter it to set the ENV Variable
    set /p AMBER_SECRET="Enter your AMBER_SECRET: "
    setx AMBER_SECRET "%AMBER_SECRET%"
)

echo Process completed.
pause

:end
endlocal
