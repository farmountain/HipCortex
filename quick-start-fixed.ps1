# HipCortex Quick Start Script for Windows PowerShell
# This script helps you get HipCortex running quickly with Docker on Windows

param(
    [Parameter()]
    [ValidateSet("full", "minimal", "dev")]
    [string]$DeploymentType = "full"
)

Write-Host "🚀 HipCortex Quick Start Setup" -ForegroundColor Green
Write-Host "==============================" -ForegroundColor Green

# Check if Docker is installed
try {
    docker --version | Out-Null
    Write-Host "✅ Docker is installed and ready" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker is not installed. Please install Docker Desktop first." -ForegroundColor Red
    Write-Host "   Visit: https://docs.docker.com/desktop/windows/" -ForegroundColor Yellow
    exit 1
}

# Check if Docker is running
try {
    docker ps | Out-Null
} catch {
    Write-Host "❌ Docker is not running. Please start Docker Desktop." -ForegroundColor Red
    exit 1
}

Write-Host "✅ Docker is running" -ForegroundColor Green

# Function to check if docker compose is available
function Test-DockerCompose {
    try {
        docker-compose --version | Out-Null
        return "docker-compose"
    } catch {
        try {
            docker compose version | Out-Null
            return "docker compose"
        } catch {
            Write-Host "❌ Docker Compose is not available." -ForegroundColor Red
            exit 1
        }
    }
}

$dockerComposeCmd = Test-DockerCompose

# Create .env file if it doesn't exist
if (-not (Test-Path ".env")) {
    Write-Host "📝 Creating .env file from template..." -ForegroundColor Yellow
    Copy-Item ".env.example" ".env"
    Write-Host "✅ Created .env file. You can customize it if needed." -ForegroundColor Green
}

# If no parameter provided, ask user
if ($DeploymentType -eq "full") {
    Write-Host ""
    Write-Host "Choose deployment option:" -ForegroundColor Cyan
    Write-Host "1) Full stack (HipCortex + PostgreSQL + Redis) - Recommended" -ForegroundColor White
    Write-Host "2) Minimal (HipCortex only with file storage)" -ForegroundColor White
    Write-Host "3) Development mode (with hot reload)" -ForegroundColor White
    
    $choice = Read-Host "Enter your choice (1-3) [1]"
    if ([string]::IsNullOrEmpty($choice)) { $choice = "1" }
    
    switch ($choice) {
        "1" { $DeploymentType = "full" }
        "2" { $DeploymentType = "minimal" }
        "3" { $DeploymentType = "dev" }
        default {
            Write-Host "❌ Invalid choice. Using full stack deployment." -ForegroundColor Red
            $DeploymentType = "full"
        }
    }
}

switch ($DeploymentType) {
    "full" {
        Write-Host "🏗️  Building and starting full stack deployment..." -ForegroundColor Blue
        if ($dockerComposeCmd -eq "docker-compose") {
            docker-compose up -d
        } else {
            docker compose up -d
        }
    }
    "minimal" {
        Write-Host "🏗️  Building and starting minimal deployment..." -ForegroundColor Blue
        docker build -t hipcortex:latest .
        docker run -d `
            --name hipcortex-minimal `
            -p 3030:3030 `
            -v hipcortex_data:/app/data `
            -e ENABLE_POSTGRES=false `
            hipcortex:latest
    }
    "dev" {
        Write-Host "🏗️  Starting development environment..." -ForegroundColor Blue
        if ($dockerComposeCmd -eq "docker-compose") {
            docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d
        } else {
            docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
        }
    }
}

Write-Host ""
Write-Host "⏳ Waiting for services to start..." -ForegroundColor Yellow
Start-Sleep -Seconds 15

# Check if HipCortex is running
try {
    $response = Invoke-RestMethod -Uri "http://localhost:3030/health" -TimeoutSec 5
    Write-Host "✅ HipCortex is running successfully!" -ForegroundColor Green
    
    Write-Host ""
    Write-Host "🌐 Access points:" -ForegroundColor Cyan
    Write-Host "   • Web Dashboard: http://localhost:3030" -ForegroundColor White
    Write-Host "   • API Health Check: http://localhost:3030/health" -ForegroundColor White
    Write-Host "   • API Documentation: http://localhost:3030/docs" -ForegroundColor White
    
    Write-Host ""
    Write-Host "🔧 Management commands:" -ForegroundColor Cyan
    if ($dockerComposeCmd -eq "docker-compose") {
        Write-Host "   • View logs: docker-compose logs -f hipcortex" -ForegroundColor White
        Write-Host "   • Stop services: docker-compose down" -ForegroundColor White
        Write-Host "   • Restart: docker-compose restart" -ForegroundColor White
    } else {
        Write-Host "   • View logs: docker compose logs -f hipcortex" -ForegroundColor White
        Write-Host "   • Stop services: docker compose down" -ForegroundColor White
        Write-Host "   • Restart: docker compose restart" -ForegroundColor White
    }
    
    Write-Host ""
    Write-Host "📚 Next steps:" -ForegroundColor Cyan
    Write-Host "   • Read the full documentation in docs/" -ForegroundColor White
    Write-Host "   • Install the VS Code extension from vscode-extension/" -ForegroundColor White
    Write-Host "   • Check the configuration in .env file" -ForegroundColor White
    
} catch {
    Write-Host "❌ HipCortex failed to start. Check the logs:" -ForegroundColor Red
    if ($dockerComposeCmd -eq "docker-compose") {
        Write-Host "   docker-compose logs hipcortex" -ForegroundColor Yellow
    } else {
        Write-Host "   docker compose logs hipcortex" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "🎉 Setup complete! Happy coding with HipCortex!" -ForegroundColor Green
