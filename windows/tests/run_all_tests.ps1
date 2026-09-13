# ==============================================================================
# Master Test Runner: Claude Code Desktop Pet Verification & Benchmark Suite
# File: tests/run_all_tests.ps1
# Executes Tiers 1-4, Hook Simulation, and Benchmarks
# ==============================================================================

param(
    [switch]$IncludeBenchmark,
    [ValidateSet("All", "1", "2", "3", "4", "HookSim")][string]$Tier = "All",
    [string]$PetBin = "",
    [string]$HookBin = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Continue"

$swTotal = [System.Diagnostics.Stopwatch]::StartNew()

Write-Host "==================================================================" -ForegroundColor Cyan
Write-Host "       CLAUDE CODE DESKTOP PET: MASTER E2E TEST RUNNER           " -ForegroundColor Cyan
Write-Host "==================================================================" -ForegroundColor Cyan
Write-Host "Timestamp: $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss'))" -ForegroundColor DarkGray

# Auto-detect binaries if not provided
if (-not $PetBin) {
    $petCandidates = @(
        "a:\CODE\claude pet\windows\target\release\sidecrab-pet.exe",
        "a:\CODE\claude pet\windows\target\release\sidecrab.exe",
        "a:\CODE\claude pet\windows\target\debug\sidecrab-pet.exe",
        "a:\CODE\claude pet\windows\target\debug\sidecrab.exe",
        "a:\CODE\claude pet\target\release\sidecrab-pet.exe",
        "a:\CODE\claude pet\target\release\sidecrab.exe",
        "a:\CODE\claude pet\target\debug\sidecrab-pet.exe",
        "a:\CODE\claude pet\target\debug\sidecrab.exe"
    )
    foreach ($p in $petCandidates) {
        if (Test-Path $p) { $PetBin = (Resolve-Path $p).Path; break }
    }
}

if (-not $HookBin) {
    $hookCandidates = @(
        "a:\CODE\claude pet\windows\target\release\sidecrab-hook.exe",
        "a:\CODE\claude pet\windows\target\debug\sidecrab-hook.exe",
        "a:\CODE\claude pet\target\release\sidecrab-hook.exe",
        "a:\CODE\claude pet\target\debug\sidecrab-hook.exe"
    )
    foreach ($h in $hookCandidates) {
        if (Test-Path $h) { $HookBin = (Resolve-Path $h).Path; break }
    }
}

Write-Host "Pet Binary:  $(if ($PetBin) { $PetBin } else { '<not detected - running contract simulation>' })" -ForegroundColor $(if ($PetBin) { "Green" } else { "Yellow" })
Write-Host "Hook Binary: $(if ($HookBin) { $HookBin } else { '<not detected - running contract simulation>' })" -ForegroundColor $(if ($HookBin) { "Green" } else { "Yellow" })

$suiteFiles = @()

if ($Tier -eq "All" -or $Tier -eq "1") {
    $suiteFiles += @{ Name = "Tier 1: Feature Coverage"; Script = "tier1_feature_coverage.ps1" }
}
if ($Tier -eq "All" -or $Tier -eq "2") {
    $suiteFiles += @{ Name = "Tier 2: Boundaries & Corners"; Script = "tier2_boundaries_corners.ps1" }
}
if ($Tier -eq "All" -or $Tier -eq "3") {
    $suiteFiles += @{ Name = "Tier 3: Cross-Feature Combinations"; Script = "tier3_cross_combinations.ps1" }
}
if ($Tier -eq "All" -or $Tier -eq "4") {
    $suiteFiles += @{ Name = "Tier 4: Real-World Scenarios"; Script = "tier4_real_world_scenarios.ps1" }
}
if ($Tier -eq "All" -or $Tier -eq "HookSim") {
    $suiteFiles += @{ Name = "Hook Simulation Suite"; Script = "hook_sim.ps1" }
}
if ($IncludeBenchmark) {
    $suiteFiles += @{ Name = "30s RAM Benchmark (<10MB limit)"; Script = "ram_benchmark.ps1" }
    $suiteFiles += @{ Name = "Idle CPU Benchmark (<=0.2% limit)"; Script = "cpu_benchmark.ps1" }
}

$suiteResults = @()
$anyFailed = $false

foreach ($suite in $suiteFiles) {
    $scriptPath = Join-Path $PSScriptRoot $suite.Script
    if (-not (Test-Path $scriptPath)) {
        Write-Host "[ERROR] Script not found: $scriptPath" -ForegroundColor Red
        $anyFailed = $true
        continue
    }

    Write-Host "`n>>> RUNNING: $($suite.Name) ($($suite.Script))" -ForegroundColor Cyan
    $suiteSw = [System.Diagnostics.Stopwatch]::StartNew()

    $paramHash = @{}
    $cmd = Get-Command $scriptPath
    if ($cmd.Parameters.ContainsKey("PetBinPath") -and $PetBin) { $paramHash["PetBinPath"] = $PetBin }
    if ($cmd.Parameters.ContainsKey("HookBinPath") -and $HookBin) { $paramHash["HookBinPath"] = $HookBin }

    $exitCode = 0
    try {
        & $scriptPath @paramHash
        $exitCode = $LASTEXITCODE
        if ($null -eq $exitCode) { $exitCode = 0 }
    } catch {
        Write-Host "  [EXCEPTION] $($_.Exception.Message)" -ForegroundColor Red
        $exitCode = 1
    }
    $suiteSw.Stop()

    $status = if ($exitCode -eq 0) { "PASS" } else { "FAIL" }
    if ($exitCode -ne 0) { $anyFailed = $true }

    $suiteResults += [PSCustomObject]@{
        Suite    = $suite.Name
        Script   = $suite.Script
        Status   = $status
        ExitCode = $exitCode
        Duration = "$([Math]::Round($suiteSw.Elapsed.TotalSeconds, 2))s"
    }
}

$swTotal.Stop()

Write-Host "`n==================================================================" -ForegroundColor Cyan
Write-Host "                   MASTER TEST SUMMARY REPORT                     " -ForegroundColor Cyan
Write-Host "==================================================================" -ForegroundColor Cyan

$suiteResults | Format-Table -AutoSize -Property Suite, Status, ExitCode, Duration

$passCount = @($suiteResults | Where-Object { $_.Status -eq "PASS" }).Count
$failCount = @($suiteResults | Where-Object { $_.Status -eq "FAIL" }).Count
$totalDuration = "$([Math]::Round($swTotal.Elapsed.TotalSeconds, 2))s"

Write-Host "Total Suites: $($suiteResults.Count) | Passed: $passCount | Failed: $failCount" -ForegroundColor $(if ($failCount -gt 0) { "Red" } else { "Green" })
Write-Host "Total Execution Time: $totalDuration" -ForegroundColor DarkGray
Write-Host "==================================================================`n" -ForegroundColor Cyan

if ($anyFailed) {
    Write-Host "[FAIL] One or more test suites failed. Check detailed output above." -ForegroundColor Red
    exit 1
}

Write-Host "[SUCCESS] All E2E test suites and verification checks passed!" -ForegroundColor Green
exit 0
