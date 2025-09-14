#!/usr/bin/env pwsh

param(
    [string]$DeploymentType = "minimal"
)

Write-Host "=== HipCortex Quick Start ===" -ForegroundColor Green
Write-Host "Deployment Type: $DeploymentType" -ForegroundColor Yellow
Write-Host ""

# Test if Docker is available
try {
    $null = docker --version 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Docker not found or not running!" -ForegroundColor Red
        exit 1
    }
    Write-Host "✅ Docker is available" -ForegroundColor Green
} catch {
    Write-Host "ERROR: Docker not found or not running!" -ForegroundColor Red
    exit 1
}

# Test docker-compose availability
$hasDockerCompose = $false
try {
    $null = docker-compose --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        $hasDockerCompose = $true
        Write-Host "✅ docker-compose is available" -ForegroundColor Green
    }
} catch {}

if (-not $hasDockerCompose) {
    Write-Host "⚠️ docker-compose not found, will use minimal deployment only" -ForegroundColor Yellow
    $DeploymentType = "minimal"
}

Write-Host ""

# Deploy based on type
if ($DeploymentType -eq "minimal") {
    Write-Host "🚀 Running minimal deployment..." -ForegroundColor Cyan
    
    # Check if image exists
    $imageExists = docker images hipcortex:latest --format "{{.Repository}}" 2>$null
    if (-not $imageExists) {
        Write-Host "📦 Building HipCortex image..." -ForegroundColor Yellow
        docker build -t hipcortex:latest .
        if ($LASTEXITCODE -ne 0) {
            Write-Host "❌ Docker build failed!" -ForegroundColor Red
            exit 1
        }
    } else {
        Write-Host "✅ HipCortex image found" -ForegroundColor Green
    }
    
    # Stop existing container if running
    docker stop hipcortex-minimal 2>$null
    docker rm hipcortex-minimal 2>$null
    
    # Run container
    Write-Host "🏃 Starting HipCortex container..." -ForegroundColor Yellow
    docker run -d --name hipcortex-minimal -p 3030:3030 hipcortex:latest
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Failed to start container!" -ForegroundColor Red
        exit 1
    }
    
} elseif ($DeploymentType -eq "full") {
    Write-Host "🏗️ Running full stack deployment..." -ForegroundColor Cyan
    docker-compose up -d --build
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Docker Compose deployment failed!" -ForegroundColor Red
        exit 1
    }
    
} else {
    Write-Host "❌ Unknown deployment type: $DeploymentType" -ForegroundColor Red
    Write-Host "Valid options: minimal, full" -ForegroundColor Yellow
    exit 1
}

# Wait for service and test health
Write-Host ""
Write-Host "⏳ Waiting for HipCortex to be ready..." -ForegroundColor Yellow

$maxAttempts = 15
$attempt = 0
$healthUrl = "http://localhost:3030/health"

do {
    $attempt++
    Start-Sleep -Seconds 2
    
    try {
        $response = Invoke-WebRequest -Uri $healthUrl -UseBasicParsing -TimeoutSec 5
        if ($response.StatusCode -eq 200) {
            Write-Host ""
            Write-Host "✅ HipCortex is ready!" -ForegroundColor Green
            break
        }
    } catch {
        Write-Host "." -NoNewline -ForegroundColor Gray
    }
    
    if ($attempt -ge $maxAttempts) {
        Write-Host ""
        Write-Host "⚠️ Health check timeout. Service may still be starting..." -ForegroundColor Yellow
        break
    }
} while ($true)

Write-Host ""
Write-Host "🎉 Deployment complete!" -ForegroundColor Green
Write-Host ""
Write-Host "📍 Access URLs:" -ForegroundColor Cyan
Write-Host "   Health Check: $healthUrl" -ForegroundColor White
Write-Host "   API Base: http://localhost:3030/api" -ForegroundColor White
Write-Host ""

if ($DeploymentType -eq "minimal") {
    Write-Host "🔧 Management commands:" -ForegroundColor Cyan
    Write-Host "   View logs: docker logs hipcortex-minimal" -ForegroundColor White
    Write-Host "   Stop: docker stop hipcortex-minimal" -ForegroundColor White
    Write-Host "   Remove: docker rm hipcortex-minimal" -ForegroundColor White
} else {
    Write-Host "🔧 Management commands:" -ForegroundColor Cyan
    Write-Host "   View logs: docker-compose logs -f" -ForegroundColor White
    Write-Host "   Stop all: docker-compose down" -ForegroundColor White
}

Write-Host ""
Write-Host "✨ Setup complete! Happy coding with HipCortex!" -ForegroundColor Green
