# ==============================================================================
# Watchdog & Physics E2E Verification
# File: tests/test_watchdog_e2e.ps1
# ==============================================================================

param(
    [string]$PetBinPath = "a:\CODE\claude pet\windows\target\release\sidecrab-pet.exe",
    [string]$HookBinPath = "a:\CODE\claude pet\windows\target\release\sidecrab-hook.exe"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "=== Verifying Watchdog and State Recovery ===" -ForegroundColor Cyan

$proc = Start-Process -FilePath $PetBinPath -PassThru
try {
    Start-Sleep -Seconds 1

    # 1. Post PreToolUse event to trigger 'tool' state
    $preEvent = '{"event":"PreToolUse","session_id":"watchdog-test","tool_name":"Bash"}'
    $preEvent | & $HookBinPath
    Start-Sleep -Milliseconds 500

    $stateFile = Join-Path $env:USERPROFILE ".sidecrab\state.json"
    $state1 = Get-Content $stateFile -Raw | ConvertFrom-Json
    Write-Host "State after PreToolUse: $($state1.state)" -ForegroundColor Yellow
    if ($state1.state -ne "tool") {
        throw "Expected state to be 'tool', got $($state1.state)"
    }

    Write-Host "Waiting 17 seconds for watchdog to trigger (15s timeout)..." -ForegroundColor Cyan
    Start-Sleep -Seconds 17

    $state2 = Get-Content $stateFile -Raw | ConvertFrom-Json
    Write-Host "State after watchdog timeout: $($state2.state)" -ForegroundColor Green
    if ($state2.state -ne "idle") {
        throw "Expected state to be reset to 'idle' by watchdog, got $($state2.state)"
    }

    Write-Host "Watchdog successfully auto-cleared stale tool state to idle!" -ForegroundColor Green
} finally {
    if (-not $proc.HasExited) {
        $proc.Kill()
        $proc.WaitForExit(2000) | Out-Null
    }
}
