# HipCortex Comprehensive Test Suite Runner
# This script executes all test phases: Unit Tests -> SIT Tests -> UAT Tests
# and provides comprehensive reporting and metrics collection

param(
    [switch]$UnitOnly,
    [switch]$if ($overallSuccess) {
    Write-Host "`n🎉 ALL TESTS PASSED! System ready for optimization phase." -ForegroundColor Green
    exit 0
} else {
    Write-Host "`n❌ Some tests failed. Review results above." -ForegroundColor Red
    exit 1
} 
    [switch]$UATOnly,
    [switch]$SkipBuild,
    [switch]$GenerateReport
)

$ErrorActionPreference = "Continue"
$StartTime = Get-Date

# Test configuration
$TestFeatures = "petgraph_backend,web-server"
$TestResults = @{
    Unit = @{ Passed = 0; Failed = 0; Duration = 0; Details = @() }
    SIT = @{ Passed = 0; Failed = 0; Duration = 0; Details = @() }
    UAT = @{ Passed = 0; Failed = 0; Duration = 0; Details = @() }
    Total = @{ Passed = 0; Failed = 0; Duration = 0 }
}

function Write-TestHeader {
    param([string]$Phase)
    Write-Host "`n" -NoNewline
    Write-Host "="*60 -ForegroundColor Cyan
    Write-Host "EXECUTING $Phase TESTS" -ForegroundColor Yellow
    Write-Host "="*60 -ForegroundColor Cyan
    Write-Host "Timestamp: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -ForegroundColor Gray
    Write-Host ""
}

function Write-TestResult {
    param([string]$TestName, [bool]$Success, [double]$Duration, [string]$Details = "")
    
    $statusColor = if ($Success) { "Green" } else { "Red" }
    $statusText = if ($Success) { "PASS" } else { "FAIL" }
    
    Write-Host "$TestName`: " -NoNewline
    Write-Host "[$statusText]" -ForegroundColor $statusColor -NoNewline
    Write-Host " (${Duration}s)" -ForegroundColor Gray
    
    if ($Details) {
        Write-Host "  Details: $Details" -ForegroundColor Yellow
    }
}

function Execute-TestPhase {
    param(
        [string]$PhaseName,
        [string]$TestCommand,
        [string]$TestPattern = ""
    )
    
    Write-TestHeader $PhaseName
    $phaseStart = Get-Date
    
    try {
        Write-Host "Executing: $TestCommand" -ForegroundColor Gray
        Write-Host ""
        
        # Execute test command and capture output
        $output = Invoke-Expression $TestCommand 2>&1
        $exitCode = $LASTEXITCODE
        
        $phaseDuration = (Get-Date - $phaseStart).TotalSeconds
        
        # Parse test results from output
        $passed = 0
        $failed = 0
        $testDetails = @()
        
        # Parse Cargo test output format
        $output | ForEach-Object {
            $line = $_.ToString()
            
            # Look for test result lines
            if ($line -match "test (\S+) \.\.\. (ok|FAILED)") {
                $testName = $matches[1]
                $result = $matches[2]
                $isSuccess = $result -eq "ok"
                
                if ($isSuccess) { $passed++ } else { $failed++ }
                $testDetails += @{
                    Name = $testName
                    Success = $isSuccess
                    Output = $line
                }
                
                Write-TestResult $testName $isSuccess 0.0
            }
            
            # Look for summary lines
            if ($line -match "test result: \w+\. (\d+) passed; (\d+) failed") {
                $passed = [int]$matches[1]
                $failed = [int]$matches[2]
            }
        }
        
        # Update results
        $TestResults[$PhaseName].Passed = $passed
        $TestResults[$PhaseName].Failed = $failed  
        $TestResults[$PhaseName].Duration = $phaseDuration
        $TestResults[$PhaseName].Details = $testDetails
        
        # Phase summary
        Write-Host ""
        Write-Host "$PhaseName Phase Summary:" -ForegroundColor Cyan
        Write-Host "  Passed: $passed" -ForegroundColor Green
        Write-Host "  Failed: $failed" -ForegroundColor $(if($failed -eq 0) { "Green" } else { "Red" })
        Write-Host "  Duration: $([math]::Round($phaseDuration, 2))s" -ForegroundColor Gray
        
        if ($failed -gt 0) {
            Write-Host ""
            Write-Host "Failed Tests:" -ForegroundColor Red
            $testDetails | Where-Object { -not $_.Success } | ForEach-Object {
                Write-Host "  - $($_.Name)" -ForegroundColor Red
            }
        }
        
        return $failed -eq 0
        
    } catch {
        Write-Host "ERROR: Test phase failed with exception: $($_.Exception.Message)" -ForegroundColor Red
        $TestResults[$PhaseName].Failed = 999
        $TestResults[$PhaseName].Duration = (Get-Date - $phaseStart).TotalSeconds
        return $false
    }
}

function Generate-TestReport {
    Write-Host "`n" -NoNewline
    Write-Host "="*60 -ForegroundColor Magenta
    Write-Host "COMPREHENSIVE TEST REPORT" -ForegroundColor Yellow
    Write-Host "="*60 -ForegroundColor Magenta
    
    $totalDuration = (Get-Date - $StartTime).TotalSeconds
    $totalPassed = ($TestResults.Unit.Passed + $TestResults.SIT.Passed + $TestResults.UAT.Passed)
    $totalFailed = ($TestResults.Unit.Failed + $TestResults.SIT.Failed + $TestResults.UAT.Failed)
    $totalTests = $totalPassed + $totalFailed
    
    Write-Host ""
    Write-Host "OVERALL RESULTS:" -ForegroundColor Cyan
    Write-Host "  Total Tests: $totalTests" -ForegroundColor White
    Write-Host "  Passed: $totalPassed" -ForegroundColor Green
    Write-Host "  Failed: $totalFailed" -ForegroundColor $(if($totalFailed -eq 0) { "Green" } else { "Red" })
    Write-Host "  Success Rate: $([math]::Round(($totalPassed / [math]::Max($totalTests, 1)) * 100, 1))%" -ForegroundColor $(if($totalFailed -eq 0) { "Green" } else { "Yellow" })
    Write-Host "  Total Duration: $([math]::Round($totalDuration, 2))s" -ForegroundColor Gray
    
    Write-Host ""
    Write-Host "PHASE BREAKDOWN:" -ForegroundColor Cyan
    @("Unit", "SIT", "UAT") | ForEach-Object {
        $phase = $_
        $result = $TestResults[$phase]
        $phaseTotal = $result.Passed + $result.Failed
        
        if ($phaseTotal -gt 0) {
            $successRate = [math]::Round(($result.Passed / $phaseTotal) * 100, 1)
            Write-Host "  $phase Tests: $($result.Passed)/$phaseTotal passed ($successRate%) in $([math]::Round($result.Duration, 2))s" -ForegroundColor White
        } else {
            Write-Host "  $phase Tests: Not executed" -ForegroundColor Gray
        }
    }
    
    # Recommendations
    Write-Host ""
    Write-Host "RECOMMENDATIONS:" -ForegroundColor Cyan
    
    if ($totalFailed -eq 0) {
        Write-Host "  ✅ All tests passed! System is ready for production deployment." -ForegroundColor Green
        Write-Host "  ✅ Consider running performance benchmarks next." -ForegroundColor Green
    } else {
    Write-Host "  ❌ $totalFailed test(s) failed. Review and fix before deployment." -ForegroundColor Red
        
        if ($TestResults.Unit.Failed -gt 0) {
            Write-Host "  🔧 Unit test failures indicate core functionality issues." -ForegroundColor Yellow
        }
        if ($TestResults.SIT.Failed -gt 0) {
            Write-Host "  🔧 SIT failures indicate integration or API issues." -ForegroundColor Yellow  
        }
        if ($TestResults.UAT.Failed -gt 0) {
            Write-Host "  🔧 UAT failures indicate user workflow issues." -ForegroundColor Yellow
        }
    }
    }
    
    Write-Host ""
    Write-Host "="*60 -ForegroundColor Magenta
}

# Main execution flow
Write-Host "HipCortex Comprehensive Test Suite" -ForegroundColor Magenta
Write-Host "Started: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -ForegroundColor Gray

# Check if we need to build first
if (-not $SkipBuild) {
    Write-Host "`nBuilding project..." -ForegroundColor Yellow
    cargo build --features $TestFeatures
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Build failed! Exiting." -ForegroundColor Red
        exit 1
    }
}

$overallSuccess = $true

# Execute test phases based on parameters
if (-not $SITOnly -and -not $UATOnly) {
    # Unit Tests
    Write-Host "`nPhase 1: Unit Tests" -ForegroundColor Magenta
    $unitSuccess = Execute-TestPhase "Unit" "cargo test --lib --features `"$TestFeatures`" -- --test-threads=1"
    $overallSuccess = $overallSuccess -and $unitSuccess
}

if (-not $UnitOnly -and -not $UATOnly) {
    # System Integration Tests (SIT)
    Write-Host "`nPhase 2: System Integration Tests" -ForegroundColor Magenta
    $sitSuccess = Execute-TestPhase "SIT" "cargo test --test sit_tests --features `"$TestFeatures`" -- --test-threads=1"
    $overallSuccess = $overallSuccess -and $sitSuccess
}

if (-not $UnitOnly -and -not $SITOnly) {
    # User Acceptance Tests (UAT)
    Write-Host "`nPhase 3: User Acceptance Tests" -ForegroundColor Magenta  
    $uatSuccess = Execute-TestPhase "UAT" "cargo test --test uat_tests --features `"$TestFeatures`" -- --test-threads=1"
    $overallSuccess = $overallSuccess -and $uatSuccess
}

# Generate comprehensive report
Generate-TestReport

# Exit with appropriate code
if ($overallSuccess) {
    Write-Host "`n🎉 ALL TESTS PASSED! System ready for optimization phase." -ForegroundColor Green
    exit 0
} else {
    Write-Host "`n❌ Some tests failed. Review results above." -ForegroundColor Red
    exit 1
}
