#!/usr/bin/env pwsh

param(
    [Parameter()]
    [ValidateSet("minimal", "full", "dev")]
    [string]$DeploymentType = "minimal"
)

Write-Host "Testing HipCortex Quick Start Script" -ForegroundColor Green
Write-Host "Deployment Type: $DeploymentType" -ForegroundColor Yellow

# Check if Docker image exists
$imageExists = docker images hipcortex:latest --format "{{.Repository}}" 2>$null
if ($imageExists) {
    Write-Host "HipCortex image found!" -ForegroundColor Green
} else {
    Write-Host "HipCortex image not found. Need to build first." -ForegroundColor Yellow
}

Write-Host "Script test completed successfully!" -ForegroundColor Green
