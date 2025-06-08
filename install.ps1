# Popcorn CLI Hackathon Installer for Windows
# Run with: powershell -ExecutionPolicy Bypass -File install.ps1

param(
    [switch]$Force = $false
)

Write-Host "🍿 Installing Popcorn CLI for Hackathon (Windows)..." -ForegroundColor Yellow

# Check if running as administrator (optional but recommended)
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "⚠️  Not running as administrator. Installation will be user-scoped." -ForegroundColor Yellow
}

# Set variables
$downloadUrl = "https://github.com/gpu-mode/popcorn-cli/releases/download/v1.1.6/popcorn-cli-windows.zip"
$tempDir = "$env:TEMP\popcorn-cli-install"
$installDir = "$env:LOCALAPPDATA\popcorn-cli"
$binaryPath = "$installDir\popcorn-cli.exe"

# Create directories
try {
    if (Test-Path $tempDir) {
        Remove-Item $tempDir -Recurse -Force
    }
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    Write-Host "✅ Created installation directories" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to create directories: $_" -ForegroundColor Red
    exit 1
}

# Download the binary
Write-Host "📥 Downloading from: $downloadUrl" -ForegroundColor Cyan
try {
    $zipPath = "$tempDir\popcorn-cli-windows.zip"
    Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing
    Write-Host "✅ Download completed" -ForegroundColor Green
} catch {
    Write-Host "❌ Download failed: $_" -ForegroundColor Red
    exit 1
}

# Extract the binary
Write-Host "📦 Extracting binary..." -ForegroundColor Cyan
try {
    Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force
    
    # Find the binary (it might be in a subdirectory)
    $binarySource = Get-ChildItem -Path $tempDir -Name "popcorn-cli.exe" -Recurse | Select-Object -First 1
    if ($binarySource) {
        $fullBinaryPath = Join-Path $tempDir $binarySource
        Copy-Item $fullBinaryPath $binaryPath -Force
        Write-Host "✅ Binary extracted and copied" -ForegroundColor Green
    } else {
        Write-Host "❌ popcorn-cli.exe not found in archive" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "❌ Extraction failed: $_" -ForegroundColor Red
    exit 1
}

# Add to PATH
Write-Host "🔧 Adding to PATH..." -ForegroundColor Cyan
try {
    $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($userPath -notlike "*$installDir*") {
        $newPath = "$installDir;$userPath"
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        Write-Host "✅ Added $installDir to user PATH" -ForegroundColor Green
        Write-Host "🔄 Please restart your terminal or PowerShell session" -ForegroundColor Yellow
    } else {
        Write-Host "✅ $installDir already in PATH" -ForegroundColor Green
    }
    
    # Also add to current session
    $env:PATH = "$installDir;$env:PATH"
} catch {
    Write-Host "⚠️  Could not modify PATH automatically: $_" -ForegroundColor Yellow
    Write-Host "Please manually add $installDir to your PATH" -ForegroundColor Yellow
}

# Cleanup
Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue

# Test installation
Write-Host "🧪 Testing installation..." -ForegroundColor Cyan
try {
    $version = & $binaryPath --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Installation successful!" -ForegroundColor Green
    } else {
        Write-Host "⚠️  Binary installed but may not be working correctly" -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠️  Could not test binary: $_" -ForegroundColor Yellow
}

# Auto-register with Discord for hackathon
Write-Host "🔐 Setting up authentication..." -ForegroundColor Cyan
$configPath = "$env:USERPROFILE\.popcorn.yaml"

# Check if already registered
if (Test-Path $configPath) {
    Write-Host "✅ Already registered! Skipping authentication setup." -ForegroundColor Green
} else {
    Write-Host "🚀 Registering with Discord for hackathon access..." -ForegroundColor Yellow
    try {
        # Add to current session PATH if not already there
        if ($env:PATH -notlike "*$installDir*") {
            $env:PATH = "$installDir;$env:PATH"
        }
        
        $result = & $binaryPath register discord
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Registration successful!" -ForegroundColor Green
        } else {
            Write-Host "⚠️  Registration failed. You can register manually later with:" -ForegroundColor Yellow
            Write-Host "   popcorn-cli register discord" -ForegroundColor White
        }
    } catch {
        Write-Host "⚠️  Registration failed: $_" -ForegroundColor Yellow
        Write-Host "   You can register manually later with: popcorn-cli register discord" -ForegroundColor White
    }
}

Write-Host ""
Write-Host "🎉 Popcorn CLI installed and configured for hackathon!" -ForegroundColor Green
Write-Host ""
Write-Host "📋 Quick Start:" -ForegroundColor Cyan
Write-Host "   1. Restart your terminal/PowerShell" -ForegroundColor White
Write-Host "   2. Submit your first solution: popcorn-cli submit <your-file>" -ForegroundColor White
Write-Host ""
Write-Host "🚀 Hackathon mode features:" -ForegroundColor Cyan
Write-Host "   - ✅ API URL pre-configured" -ForegroundColor White
Write-Host "   - ✅ Discord authentication set up" -ForegroundColor White
Write-Host "   - ✅ Only 'test' and 'benchmark' modes available" -ForegroundColor White
Write-Host "   - ✅ Ready to use immediately!" -ForegroundColor White
Write-Host ""
Write-Host "💡 Need help? Run: popcorn-cli --help" -ForegroundColor White
Write-Host ""
Write-Host "📁 Installation location: $installDir" -ForegroundColor Gray 