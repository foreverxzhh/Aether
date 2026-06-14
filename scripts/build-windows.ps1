# 构建 Aether Windows .NET SDK
#
# 前置条件:
#   1. Rust: rustup target add x86_64-pc-windows-msvc
#   2. .NET SDK: dotnet --version >= 6.0
#
# 运行:
#   pwsh scripts/build-windows.ps1

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$DotNetDir = "$ProjectRoot/sdks/dotnet"
$RuntimesDir = "$DotNetDir/runtimes/win-x64/native"

Write-Host "🔨 构建 Aether Windows SDK" -ForegroundColor Green
Write-Host ""

# 1. 编译 Rust native 库
Write-Host "📦 编译 native 库 (agent_bindings.dll)..." -ForegroundColor Yellow
cargo build -p agent-bindings --release

# 复制 DLL 到 NuGet 运行时目录
New-Item -ItemType Directory -Force -Path $RuntimesDir | Out-Null
Copy-Item "$ProjectRoot/target/release/agent_bindings.dll" "$RuntimesDir/agent_bindings.dll"
Write-Host "  ✅ $RuntimesDir/agent_bindings.dll" -ForegroundColor Green

# 2. 打包 NuGet
Write-Host ""
Write-Host "📦 打包 NuGet 包..." -ForegroundColor Yellow
cd $DotNetDir
dotnet pack -c Release -o ./nupkg
Write-Host "  ✅ NuGet: $DotNetDir/nupkg/Aether.Sdk.0.1.0.nupkg" -ForegroundColor Green

Write-Host ""
Write-Host "✅ Windows SDK 构建完成" -ForegroundColor Green
Write-Host ""
Write-Host "用法:"
Write-Host "  dotnet add package Aether.Sdk --source $DotNetDir/nupkg"
Write-Host ""
Write-Host "或直接引用 DLL:"
Write-Host '  $env:Path += ";$pwd\sdks\dotnet\runtimes\win-x64\native"'
Write-Host "  dotnet run"
