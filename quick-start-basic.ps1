param([string]$DeploymentType = "minimal")

Write-Host "HipCortex Quick Start" -ForegroundColor Green
Write-Host "Deployment Type: $DeploymentType" -ForegroundColor Yellow

# Check Docker
docker --version
if ($LASTEXITCODE -ne 0) {
    Write-Host "Docker not available!" -ForegroundColor Red
    exit 1
}

# Check if minimal container is already running
$existing = docker ps -q -f name=hipcortex-minimal
if ($existing) {
    Write-Host "Stopping existing container..." -ForegroundColor Yellow
    docker stop $existing
    docker rm $existing
}

# Run the container
Write-Host "Starting HipCortex container..." -ForegroundColor Yellow
docker run -d --name hipcortex-minimal -p 3030:3030 hipcortex:latest

# Test health
Start-Sleep 5
Write-Host "Testing health endpoint..." -ForegroundColor Yellow
$response = Invoke-WebRequest -Uri "http://localhost:3030/health" -UseBasicParsing
Write-Host "Health check response: $($response.Content)" -ForegroundColor Green

Write-Host "Deployment complete!" -ForegroundColor Green
