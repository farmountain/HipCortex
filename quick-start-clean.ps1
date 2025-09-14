#!/usr/bin/env pwsh

<#
.SYNOPSIS
    HipCortex Quick Start - Docker/Podman deployment
.DESCRIPTION
    This script provides automated deployment options for HipCortex using Docker or Podman.
.PARAMETER DeploymentType
    Type of deployment: 'minimal', 'full', or 'dev'
#>

param(
    [Parameter()]
    [ValidateSet("minimal", "full", "dev")]
    [string]$DeploymentType
)

function Test-DockerCompose {
    Write-Host "🔍 Checking for Docker Compose..." -ForegroundColor Yellow
    
    # Test docker compose (newer syntax)
    try {
        $null = docker compose version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Found 'docker compose'" -ForegroundColor Green
            return "docker compose"
        }
    } catch {}
    
    # Test docker-compose (legacy)
    try {
        $null = docker-compose --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Found 'docker-compose'" -ForegroundColor Green
            return "docker-compose"
        }
    } catch {}
    
    # Test podman-compose
    try {
        $null = podman-compose --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Found 'podman-compose'" -ForegroundColor Green
            return "podman-compose"
        }
    } catch {}
    
    Write-Host "❌ No container orchestration tool found!" -ForegroundColor Red
    Write-Host "Please install one of:" -ForegroundColor Yellow
    Write-Host "  - Docker Desktop with Compose" -ForegroundColor White
    Write-Host "  - Podman with podman-compose" -ForegroundColor White
    exit 1
}

$dockerComposeCmd = Test-DockerCompose

# Create .env file if it doesn't exist
if (-not (Test-Path ".env")) {
    Write-Host "📝 Creating .env file from template..." -ForegroundColor Yellow
    Copy-Item ".env.example" ".env"
    Write-Host "✅ Created .env file. You can customize it if needed." -ForegroundColor Green
}

# Prompt for deployment type if not provided
if (-not $DeploymentType) {
    Write-Host ""
    Write-Host "Choose deployment option:" -ForegroundColor Cyan
    Write-Host "1) Full stack - HipCortex + PostgreSQL + Redis - Recommended" -ForegroundColor White
    Write-Host "2) Minimal - HipCortex only with file storage" -ForegroundColor White
    Write-Host "3) Development mode - with hot reload" -ForegroundColor White
    
    $choice = Read-Host "Enter your choice (1-3) [1]"
    if ([string]::IsNullOrEmpty($choice)) { $choice = "1" }
    
    switch ($choice) {
        "1" { $DeploymentType = "full" }
        "2" { $DeploymentType = "minimal" }
        "3" { $DeploymentType = "dev" }
        default { 
            Write-Host "Invalid choice. Using full deployment." -ForegroundColor Yellow
            $DeploymentType = "full" 
        }
    }
}

Write-Host ""
Write-Host "🚀 Starting HipCortex deployment: $DeploymentType" -ForegroundColor Green
Write-Host ""

# Deploy based on type
switch ($DeploymentType) {
    "minimal" {
        Write-Host "📦 Building HipCortex image..." -ForegroundColor Yellow
        & docker build -t hipcortex:latest .
        if ($LASTEXITCODE -ne 0) {
            Write-Host "❌ Docker build failed!" -ForegroundColor Red
            exit 1
        }
        
        Write-Host "🏃 Running minimal deployment..." -ForegroundColor Yellow
        & docker run -d --name hipcortex-minimal -p 3030:3030 hipcortex:latest
        if ($LASTEXITCODE -ne 0) {
            Write-Host "❌ Failed to start HipCortex container!" -ForegroundColor Red
            exit 1
        }
        
        $healthUrl = "http://localhost:3030/health"
    }
    
    "full" {
        Write-Host "🏗️ Building and starting full stack..." -ForegroundColor Yellow
        & $dockerComposeCmd up -d --build
        if ($LASTEXITCODE -ne 0) {
            Write-Host "❌ Docker Compose deployment failed!" -ForegroundColor Red
            exit 1
        }
        
        $healthUrl = "http://localhost:3030/health"
    }
    
    "dev" {
        Write-Host "🛠️ Starting development environment..." -ForegroundColor Yellow
        & $dockerComposeCmd -f docker-compose.yml -f docker-compose.dev.yml up -d --build
        if ($LASTEXITCODE -ne 0) {
            Write-Host "❌ Development deployment failed!" -ForegroundColor Red
            exit 1
        }
        
        $healthUrl = "http://localhost:3030/health"
    }
}

# Wait for service to be ready
Write-Host ""
Write-Host "⏳ Waiting for HipCortex to be ready..." -ForegroundColor Yellow
$maxAttempts = 30
$attempt = 0

do {
    $attempt++
    try {
        $response = Invoke-WebRequest -Uri $healthUrl -UseBasicParsing -TimeoutSec 5
        if ($response.StatusCode -eq 200) {
            Write-Host "✅ HipCortex is ready!" -ForegroundColor Green
            break
        }
    } catch {
        Write-Host "." -NoNewline -ForegroundColor Gray
    }
    
    Start-Sleep -Seconds 2
    
    if ($attempt -ge $maxAttempts) {
        Write-Host ""
        Write-Host "⚠️ Health check timeout. HipCortex may still be starting..." -ForegroundColor Yellow
        break
    }
} while ($true)

Write-Host ""
Write-Host "🎉 HipCortex deployment complete!" -ForegroundColor Green
Write-Host ""
Write-Host "📍 Access URLs:" -ForegroundColor Cyan
Write-Host "   Health Check: $healthUrl" -ForegroundColor White
Write-Host "   API Base: http://localhost:3030/api" -ForegroundColor White
Write-Host ""

# Show next steps
Write-Host "🔧 Next steps:" -ForegroundColor Cyan
switch ($DeploymentType) {
    "minimal" {
        Write-Host "   • Test the API: curl $healthUrl" -ForegroundColor White
        Write-Host "   • Stop: docker stop hipcortex-minimal" -ForegroundColor White
        Write-Host "   • Remove: docker rm hipcortex-minimal" -ForegroundColor White
    }
    default {
        Write-Host "   • Test the API: curl $healthUrl" -ForegroundColor White
        Write-Host "   • View logs: $dockerComposeCmd logs -f" -ForegroundColor White
        Write-Host "   • Stop all: $dockerComposeCmd down" -ForegroundColor White
        Write-Host "   • Stop and remove data: $dockerComposeCmd down -v" -ForegroundColor White
    }
}

Write-Host ""
Write-Host "✨ Setup complete! Happy coding with HipCortex!" -ForegroundColor Green
