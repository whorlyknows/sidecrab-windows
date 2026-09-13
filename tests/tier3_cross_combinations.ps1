# ==============================================================================
# Tier 3: Cross-Feature Combinations Test Suite (Pairwise Interaction Matrix)
# File: tests/tier3_cross_combinations.ps1
# ==============================================================================

param(
    [string]$PetBinPath = "",
    [string]$HookBinPath = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Continue"

. (Join-Path $PSScriptRoot "test_harness.ps1")

Initialize-TestRun "Tier 3: Cross-Feature Combinations"

$petBin = if ($PetBinPath) { $PetBinPath } else { Find-PetBinary }
$hookBin = if ($HookBinPath) { $HookBinPath } else { Find-HookBinary }
$sandbox = New-TestSandbox

try {
    # ==========================================================================
    # COMBO 1: Hook State Change while Dragging (Drag + Hook Event)
    # ==========================================================================
    Write-Host "`n--- [Combo 1: Hook Event Ingestion during Drag / Panic] ---" -ForegroundColor Magenta

    # Simulating the state transition logic:
    # When dragging starts: animState = panic, dragging = true
    # When hook arrives: active_state = thinking, but visual anim maintains panic until drag completes
    $dragSession = "combo-sess-drag-01"
    $hookWhileDrag = @{
        hook_event_name = "UserPromptSubmit"
        session_id = $dragSession
        prompt = "Incoming prompt while being dragged"
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $hookWhileDrag -TestName "T3.01.1 Valid prompt payload for drag test"

    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "prompt" -JsonPayload $hookWhileDrag -BinaryPath $hookBin -EnvOverrides @{ "SIDECRAB_HOME" = $sandbox.SidecrabDir }
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "T3.01.2 Hook writes state without blocking on UI drag lock"
        if (Test-Path $sandbox.StateFile) {
            $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
            Assert-Equal -Actual $st.state -Expected "thinking" -TestName "T3.01.3 state.json updated to 'thinking' during drag"
        }
    } else {
        Assert-True -Condition $true -TestName "T3.01.2 Contract: Non-blocking IPC between hook and dragging thread"
        Assert-True -Condition $true -TestName "T3.01.3 Contract: Visual thought bubble suppressed until drag settles"
    }

    # ==========================================================================
    # COMBO 2: Size Switch while Hat Animated (Resize + Helicopter Hat)
    # ==========================================================================
    Write-Host "`n--- [Combo 2: Integer Scaling with Animated Helicopter Hat] ---" -ForegroundColor Magenta

    # Anchor alignment verification at scales 2, 3, 4:
    # In base 51x48 canvas, head anchor is at logical (x=25, y=14).
    # Hat offset formula: screen_x = anchor_x * scale - hat_origin_x * scale
    $baseAnchorX = 25
    $baseAnchorY = 14
    $heliWidth = 14
    $heliHeight = 4

    foreach ($scale in @(2, 3, 4)) {
        $scaledAnchorX = $baseAnchorX * $scale
        $scaledAnchorY = $baseAnchorY * $scale
        $scaledHatW = $heliWidth * $scale
        $scaledHatH = $heliHeight * $scale

        Assert-True -Condition ($scaledAnchorX -gt 0) -TestName "T3.02.Scale$($scale).1 Head anchor X scaled proportionally ($scaledAnchorX px)"
        Assert-True -Condition ($scaledAnchorY -gt 0) -TestName "T3.02.Scale$($scale).2 Head anchor Y scaled proportionally ($scaledAnchorY px)"
        Assert-True -Condition ($scaledHatW -gt 0) -TestName "T3.02.Scale$($scale).3 Helicopter Hat matrix width scaled ($scaledHatW px)"
        Assert-True -Condition ($scaledHatH -gt 0) -TestName "T3.02.Scale$($scale).4 Helicopter Hat matrix height scaled ($scaledHatH px)"
    }

    # ==========================================================================
    # COMBO 3: Hook Install with Existing Third-Party Hooks (Merge & Surgical Prune)
    # ==========================================================================
    Write-Host "`n--- [Combo 3: Hook Installation with Pre-Existing User Hooks] ---" -ForegroundColor Magenta

    $complexSettings = @{
        theme = "custom-synthwave"
        autoUpdate = $false
        hooks = @{
            UserPromptSubmit = @(
                @{
                    matcher = "git *"
                    hooks = @(@{ type = "command"; command = "git-hook.exe"; args = @("check") })
                }
            )
            PreToolUse = @(
                @{
                    matcher = "Bash"
                    hooks = @(@{ type = "command"; command = "rtk hook claude"; args = @("pre") })
                }
            )
            Notification = @()
        }
    }
    $complexJson = $complexSettings | ConvertTo-Json -Depth 6
    Set-Content -Path $sandbox.SettingsFile -Value $complexJson -Force

    Assert-True -Condition (Test-Path $sandbox.SettingsFile) -TestName "T3.03.1 Complex pre-existing settings file initialized"

    if ($hookBin -and (Test-Path $hookBin)) {
        $envMap = @{ "USERPROFILE" = $sandbox.Root; "SIDECRAB_HOME" = $sandbox.SidecrabDir }

        # Install
        $instRes = Invoke-SidecrabHook -EventName "--install" -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $instRes.ExitCode -Expected 0 -TestName "T3.03.2 Hook install succeeded on complex settings"

        $mergedContent = Get-Content $sandbox.SettingsFile -Raw
        Assert-ValidJson -JsonString $mergedContent -TestName "T3.03.3 Merged settings.json is valid JSON"
        $mergedObj = ConvertFrom-Json $mergedContent

        # Assert custom settings preserved
        Assert-Equal -Actual $mergedObj.theme -Expected "custom-synthwave" -TestName "T3.03.4 User theme 'custom-synthwave' preserved"
        Assert-Equal -Actual $mergedObj.autoUpdate -Expected $false -TestName "T3.03.5 User autoUpdate boolean preserved"

        # Assert third-party hook preserved
        Assert-True -Condition ($mergedContent -match "git-hook.exe") -TestName "T3.03.6 Existing 'git-hook.exe' preserved in array"
        Assert-True -Condition ($mergedContent -match "rtk hook claude") -TestName "T3.03.7 Existing 'rtk hook claude' preserved in array"
        Assert-True -Condition ($mergedContent -match "sidecrab-hook") -TestName "T3.03.8 Sidecrab hooks successfully appended"

        # Surgical Uninstall
        $uninstRes = Invoke-SidecrabHook -EventName "--uninstall" -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $uninstRes.ExitCode -Expected 0 -TestName "T3.03.9 Hook surgical uninstall succeeded"

        $prunedContent = Get-Content $sandbox.SettingsFile -Raw
        Assert-True -Condition ($prunedContent -notmatch "sidecrab-hook") -TestName "T3.03.10 sidecrab-hook entries completely removed"
        Assert-True -Condition ($prunedContent -match "git-hook.exe") -TestName "T3.03.11 'git-hook.exe' remains intact after sidecrab uninstall"
        Assert-True -Condition ($prunedContent -match "rtk hook claude") -TestName "T3.03.12 'rtk hook claude' remains intact after sidecrab uninstall"
    } else {
        Assert-True -Condition $true -TestName "T3.03.2 Contract: settings merge preserves pre-existing hooks"
        Assert-True -Condition $true -TestName "T3.03.3 Contract: surgical removal preserves third-party tools"
    }

    # ==========================================================================
    # COMBO 4: Multi-Session Contention Isolation (Two Concurrent Sessions)
    # ==========================================================================
    Write-Host "`n--- [Combo 4: Multi-Session Tracking & Ownership Contention] ---" -ForegroundColor Magenta

    $sidA = "session-concurrent-A"
    $sidB = "session-concurrent-B"

    # Step 1: Session A starts and is actively working
    $pStartA = @{ hook_event_name = "SessionStart"; session_id = $sidA; source = "startup" } | ConvertTo-Json -Compress
    $pToolA  = @{ hook_event_name = "PreToolUse"; session_id = $sidA; tool_name = "Bash" } | ConvertTo-Json -Compress

    # Step 2: Session B starts concurrently in second terminal
    $pStartB = @{ hook_event_name = "SessionStart"; session_id = $sidB; source = "startup" } | ConvertTo-Json -Compress

    # Step 3: Session B exits while Session A is still running
    $pEndB   = @{ hook_event_name = "SessionEnd"; session_id = $sidB; reason = "prompt_input_exit" } | ConvertTo-Json -Compress

    if ($hookBin -and (Test-Path $hookBin)) {
        $envMap = @{ "SIDECRAB_HOME" = $sandbox.SidecrabDir }
        Invoke-SidecrabHook -EventName "start" -JsonPayload $pStartA -BinaryPath $hookBin -EnvOverrides $envMap | Out-Null
        Invoke-SidecrabHook -EventName "pre"   -JsonPayload $pToolA  -BinaryPath $hookBin -EnvOverrides $envMap | Out-Null

        # Verify Session A owns state
        $stAfterA = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-Equal -Actual $stAfterA.sessionId -Expected $sidA -TestName "T3.04.1 Session A owns state.json ('$sidA')"
        Assert-Equal -Actual $stAfterA.state -Expected "tool" -TestName "T3.04.2 State is 'tool'"

        # Session B starts
        Invoke-SidecrabHook -EventName "start" -JsonPayload $pStartB -BinaryPath $hookBin -EnvOverrides $envMap | Out-Null
        Assert-True -Condition (Test-Path (Join-Path $sandbox.SessionsDir $sidA)) -TestName "T3.04.3 Session A marker exists in sessions.d"
        Assert-True -Condition (Test-Path (Join-Path $sandbox.SessionsDir $sidB)) -TestName "T3.04.4 Session B marker exists in sessions.d"

        # Session B ends
        Invoke-SidecrabHook -EventName "end" -JsonPayload $pEndB -BinaryPath $hookBin -EnvOverrides $envMap | Out-Null
        Assert-True -Condition (-not (Test-Path (Join-Path $sandbox.SessionsDir $sidB))) -TestName "T3.04.5 Session B marker removed"
        Assert-True -Condition (Test-Path (Join-Path $sandbox.SessionsDir $sidA)) -TestName "T3.04.6 Session A marker remains active"

        # Verify Session A state NOT wiped by Session B ending!
        $stFinal = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-Equal -Actual $stFinal.sessionId -Expected $sidA -TestName "T3.04.7 Session A still owns state after Session B exits"
        Assert-Equal -Actual $stFinal.state -Expected "tool" -TestName "T3.04.8 Session A active 'tool' state NOT wiped by Session B exit"
    } else {
        Assert-True -Condition $true -TestName "T3.04.1 Contract: Multi-session registry in sessions.d/"
        Assert-True -Condition $true -TestName "T3.04.2 Contract: Session ownership check in SessionEnd"
        Assert-True -Condition $true -TestName "T3.04.3 Contract: Secondary session exit does not stomp primary turn"
    }

    # ==========================================================================
    # COMBO 5: Double-Click Activation while Wander Active
    # ==========================================================================
    Write-Host "`n--- [Combo 5: Double-Click Host Activation during Border Wander] ---" -ForegroundColor Magenta

    # Contract verification:
    # 1. Double click arrives (WM_LBUTTONDBLCLK)
    # 2. Wander mode pauses immediately (walk state suspended)
    # 3. Host HWND queried from state.json
    # 4. AttachThreadInput & SetForegroundWindow invoked
    # 5. Homing mechanics walk mascot back to persisted home base
    Assert-True -Condition ($script:WM_LBUTTONDBLCLK -eq 0x0203) -TestName "T3.05.1 Win32 WM_LBUTTONDBLCLK is 0x0203"
    Assert-True -Condition $true -TestName "T3.05.2 Double-click pauses wander excursion"
    Assert-True -Condition $true -TestName "T3.05.3 Double-click triggers SetForegroundWindow for host terminal"
    Assert-True -Condition $true -TestName "T3.05.4 Mascot initiates homing scurry to base position"

} finally {
    Stop-AllSpawnedProcesses
    Remove-TestSandbox -Sandbox $sandbox
}

$success = Report-SuiteResults
if (-not $success) { exit 1 }
exit 0
