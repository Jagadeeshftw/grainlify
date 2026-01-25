@echo off
echo Checking if Rust is installed...
rustc --version >nul 2>&1
if %errorlevel% neq 0 (
    echo Rust is not installed. Please install Rust first:
    echo 1. Download and install Rust from https://www.rust-lang.org/tools/install
    echo 2. Or run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs ^| sh
    exit /b 1
)

echo Checking if Soroban CLI is installed...
soroban --version >nul 2>&1
if %errorlevel% neq 0 (
    echo Installing Soroban CLI...
    cargo install --locked soroban-cli
)

echo Navigating to contract directory...
cd /d "c:\Users\user\Desktop\GIT\grainlify\contracts\grainlify-core"

echo Building the contract...
cargo build

if %errorlevel% equ 0 (
    echo.
    echo Contract built successfully!
    echo Running tests...
    cargo test
) else (
    echo Build failed. Please check the errors above.
)