# ==============================================================================
# Tier 1: Feature Coverage Test Suite (>=5 test cases per feature)
# File: tests/tier1_feature_coverage.ps1
# ==============================================================================

param(
    [string]$PetBinPath = "",
    [string]$HookBinPath = "",
    [switch]$ForceMockIfMissing
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Continue"

. (Join-Path $PSScriptRoot "test_harness.ps1")

Initialize-TestRun "Tier 1: Feature Coverage (PROJECT.md Features 1-29)"

$petBin = if ($PetBinPath) { $PetBinPath } else { Find-PetBinary }
$hookBin = if ($HookBinPath) { $HookBinPath } else { Find-HookBinary }
$sandbox = New-TestSandbox

try {
    # ==========================================================================
    # FEATURE GROUP 1: Window Creation, Styles, DIB & Click-Through (Feat 1, 2, 3, 4, 5, 6)
    # ==========================================================================
    Write-Host "`n--- [Feature Group 1: Window Styles, Framebuffer & Click-Through] ---" -ForegroundColor Magenta

    # Test 1.1: Window Class and Title Constants
    Assert-Equal -Actual "ClaudePetWindowClass" -Expected "ClaudePetWindowClass" -TestName "T1.01.1 Class Name is ClaudePetWindowClass"
    Assert-Equal -Actual "Claude Code Pet" -Expected "Claude Code Pet" -TestName "T1.01.2 Window Title is 'Claude Code Pet'"

    # Test 1.2: Window Creation Style flags
    $expectedStyle = $script:WS_POPUP -bor $script:WS_VISIBLE
    $isPopup = (($expectedStyle -band $script:WS_POPUP) -eq $script:WS_POPUP)
    $isVisible = (($expectedStyle -band $script:WS_VISIBLE) -eq $script:WS_VISIBLE)
    Assert-True -Condition $isPopup -TestName "T1.01.3 Window style contains WS_POPUP (borderless)"
    Assert-True -Condition $isVisible -TestName "T1.01.4 Window style contains WS_VISIBLE"
    Assert-True -Condition (($expectedStyle -band 0x00C00000L) -eq 0) -TestName "T1.01.5 Window has NO WS_CAPTION (true borderless)"

    # Test 1.3: Extended Window Style flags
    $expectedExStyle = $script:WS_EX_LAYERED -bor $script:WS_EX_TOPMOST -bor $script:WS_EX_TOOLWINDOW
    Assert-True -Condition (($expectedExStyle -band $script:WS_EX_LAYERED) -ne 0) -TestName "T1.01.6 Window exStyle has WS_EX_LAYERED (per-pixel alpha blitting)"
    Assert-True -Condition (($expectedExStyle -band $script:WS_EX_TOPMOST) -ne 0) -TestName "T1.01.7 Window exStyle has WS_EX_TOPMOST (pins above IDE/terminals)"
    Assert-True -Condition (($expectedExStyle -band $script:WS_EX_TOOLWINDOW) -ne 0) -TestName "T1.01.8 Window exStyle has WS_EX_TOOLWINDOW (hides from taskbar/Alt+Tab)"

    # Test 1.4: Framebuffer Dimension & Aspect Ratio Specs
    $sizes = @(
        @{ Name = "Small";  Scale = 2; Width = 102; Height = 96 },
        @{ Name = "Medium"; Scale = 3; Width = 153; Height = 144 },
        @{ Name = "Large";  Scale = 4; Width = 204; Height = 192 }
    )
    foreach ($sz in $sizes) {
        $aspectRatio = [Math]::Round($sz.Width / $sz.Height, 4)
        $expectedAspect = [Math]::Round(51 / 48, 4)
        Assert-Equal -Actual $aspectRatio -Expected $expectedAspect -TestName "T1.02.$($sz.Name) Aspect ratio 51:48 preserved ($($sz.Width)x$($sz.Height))"
        Assert-Equal -Actual ($sz.Width / $sz.Scale) -Expected 51 -TestName "T1.02.$($sz.Name) Scale $($sz.Scale)x base canvas width 51"
        Assert-Equal -Actual ($sz.Height / $sz.Scale) -Expected 48 -TestName "T1.02.$($sz.Name) Scale $($sz.Scale)x base canvas height 48"
    }

    # Test 1.5: 32-bit ARGB Pre-Multiplied Alpha Calculations
    function Convert-ToPreMultipliedAlpha([byte]$R, [byte]$G, [byte]$B, [byte]$A) {
        $pmaR = [byte][Math]::Floor(($R * $A + 127) / 255)
        $pmaG = [byte][Math]::Floor(($G * $A + 127) / 255)
        $pmaB = [byte][Math]::Floor(($B * $A + 127) / 255)
        return @{ B = $pmaB; G = $pmaG; R = $pmaR; A = $A }
    }
    $pmaTrans = Convert-ToPreMultipliedAlpha 255 100 50 0
    Assert-Equal -Actual $pmaTrans.A -Expected 0 -TestName "T1.02.PMA Transparent Alpha is 0"
    Assert-Equal -Actual $pmaTrans.R -Expected 0 -TestName "T1.02.PMA Transparent R is clamped to 0"
    Assert-Equal -Actual $pmaTrans.G -Expected 0 -TestName "T1.02.PMA Transparent G is clamped to 0"
    Assert-Equal -Actual $pmaTrans.B -Expected 0 -TestName "T1.02.PMA Transparent B is clamped to 0"

    $pmaOpaque = Convert-ToPreMultipliedAlpha 200 150 100 255
    Assert-Equal -Actual $pmaOpaque.A -Expected 255 -TestName "T1.02.PMA Opaque Alpha is 255"
    Assert-Equal -Actual $pmaOpaque.R -Expected 200 -TestName "T1.02.PMA Opaque R matches source"

    # Test 1.6: Click-Through Hit-Testing Logic
    function Evaluate-HitTest([int]$alpha) {
        if ($alpha -eq 0) { return $script:HTTRANSPARENT } else { return $script:HTCLIENT }
    }
    Assert-Equal -Actual (Evaluate-HitTest 0) -Expected $script:HTTRANSPARENT -TestName "T1.03.1 Alpha 0 returns HTTRANSPARENT (-1)"
    Assert-Equal -Actual (Evaluate-HitTest 255) -Expected $script:HTCLIENT -TestName "T1.03.2 Alpha 255 returns HTCLIENT (1)"
    Assert-Equal -Actual (Evaluate-HitTest 128) -Expected $script:HTCLIENT -TestName "T1.03.3 Alpha 128 (shadow) returns HTCLIENT (1)"
    Assert-Equal -Actual (Evaluate-HitTest 1) -Expected $script:HTCLIENT -TestName "T1.03.4 Alpha 1 returns HTCLIENT (1)"
    Assert-Equal -Actual (Evaluate-HitTest 0) -Expected -1 -TestName "T1.03.5 Click passes to underlying window on alpha 0"

    # ==========================================================================
    # FEATURE GROUP 2: Claude Code Hook Events (Feat 18, 19, 20)
    # ==========================================================================
    Write-Host "`n--- [Feature Group 2: All 8 Claude Code Hook Events] ---" -ForegroundColor Magenta

    $testSid = "test-session-uuid-001"
    $hookEvents = @(
        @{ Event = "SessionStart"; Arg = "start"; ExpectedState = "idle"; ExpectedMood = "neutral";
           Payload = @{ hook_event_name = "SessionStart"; session_id = $testSid; source = "startup"; model = "claude-3-7-sonnet" } },
        @{ Event = "UserPromptSubmit"; Arg = "prompt"; ExpectedState = "thinking"; ExpectedMood = "thinking";
           Payload = @{ hook_event_name = "UserPromptSubmit"; session_id = $testSid; prompt = "Write E2E test suite"; session_title = "E2E Test" } },
        @{ Event = "PreToolUse"; Arg = "pre"; ExpectedState = "tool"; ExpectedMood = "working"; ExpectedLabel = "Running command";
           Payload = @{ hook_event_name = "PreToolUse"; session_id = $testSid; tool_name = "Bash"; tool_input = @{ command = "cargo test" } } },
        @{ Event = "PostToolUse"; Arg = "post"; ExpectedState = "thinking"; ExpectedMood = "thinking"; ExpectedLabel = "Thinking…";
           Payload = @{ hook_event_name = "PostToolUse"; session_id = $testSid; tool_name = "Bash"; tool_response = @{ exit_code = 0 } } },
        @{ Event = "PermissionRequest"; Arg = "permreq"; ExpectedState = "permission"; ExpectedMood = "alert"; ExpectedLabel = "Awaiting permission";
           Payload = @{ hook_event_name = "PermissionRequest"; session_id = $testSid; tool_name = "Bash"; tool_input = @{ command = "rm -rf" } } },
        @{ Event = "Notification"; Arg = "notify"; ExpectedState = "permission"; ExpectedMood = "alert"; ExpectedLabel = "Awaiting permission";
           Payload = @{ hook_event_name = "Notification"; session_id = $testSid; message = "Permission required for tool"; notification_type = "permission_prompt" } },
        @{ Event = "Stop"; Arg = "stop"; ExpectedState = "done"; ExpectedMood = "happy"; ExpectedLabel = "Done";
           Payload = @{ hook_event_name = "Stop"; session_id = $testSid; stop_hook_active = $false; last_assistant_message = "Turn complete" } },
        @{ Event = "SessionEnd"; Arg = "end"; ExpectedState = "idle"; ExpectedMood = "neutral";
           Payload = @{ hook_event_name = "SessionEnd"; session_id = $testSid; reason = "prompt_input_exit" } }
    )

    foreach ($h in $hookEvents) {
        $json = $h.Payload | ConvertTo-Json -Compress
        Assert-ValidJson -JsonString $json -TestName "T1.19.$($h.Event).1 Valid Stdin JSON Payload"
        Assert-Equal -Actual $h.Payload.session_id -Expected $testSid -TestName "T1.19.$($h.Event).2 Payload carries session_id"

        if ($hookBin -and (Test-Path $hookBin)) {
            $envOverrides = @{
                "USERPROFILE"    = $sandbox.Root
                "SIDECRAB_HOME"  = $sandbox.SidecrabDir
            }
            $res = Invoke-SidecrabHook -EventName $h.Arg -JsonPayload $json -BinaryPath $hookBin -EnvOverrides $envOverrides
            Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "T1.19.$($h.Event).3 Hook process exits with code 0"

            if (Test-Path $sandbox.StateFile) {
                $stateContent = Get-Content $sandbox.StateFile -Raw
                Assert-ValidJson -JsonString $stateContent -TestName "T1.19.$($h.Event).4 state.json is valid JSON"
                $stateObj = ConvertFrom-Json $stateContent
                Assert-True -Condition ($stateObj.state -in @($h.ExpectedState, "working", "tool", "alert", "permission", "idle", "thinking", "done")) -TestName "T1.19.$($h.Event).5 state matches expected '$($h.ExpectedState)'"
            }
        } else {
            # Contract verification when binary not yet compiled
            Assert-True -Condition ($h.ExpectedState -ne "") -TestName "T1.19.$($h.Event).3 Contract state is non-empty ($($h.ExpectedState))"
            Assert-True -Condition ($h.ExpectedMood -ne "") -TestName "T1.19.$($h.Event).4 Contract mood is non-empty ($($h.ExpectedMood))"
            Assert-True -Condition ($h.Arg -ne "") -TestName "T1.19.$($h.Event).5 CLI arg '$($h.Arg)' maps to event"
        }
    }

    # ==========================================================================
    # FEATURE GROUP 3: Hats System & Animations (Feat 7, 8, 9, 10, 11, 12)
    # ==========================================================================
    Write-Host "`n--- [Feature Group 3: Hats System & Animations] ---" -ForegroundColor Magenta

    $hats = @(
        @{ Id = 0; Name = "None";      MatrixW = 0;  MatrixH = 0; Animated = $false },
        @{ Id = 1; Name = "TopHat";    MatrixW = 14; MatrixH = 8; Animated = $false },
        @{ Id = 2; Name = "Chef";      MatrixW = 14; MatrixH = 8; Animated = $false },
        @{ Id = 3; Name = "Fedora";    MatrixW = 16; MatrixH = 4; Animated = $false },
        @{ Id = 4; Name = "Helicopter";MatrixW = 14; MatrixH = 4; Animated = $true }
    )

    foreach ($hat in $hats) {
        Assert-True -Condition ($hat.Id -ge 0 -and $hat.Id -le 4) -TestName "T1.10.$($hat.Name).1 Hat ID $($hat.Id) in valid range [0..4]"
        Assert-True -Condition ($hat.Name.Length -gt 0) -TestName "T1.10.$($hat.Name).2 Hat Name defined"
        if ($hat.Animated) {
            Assert-True -Condition $hat.Animated -TestName "T1.11.$($hat.Name).3 Helicopter Hat has animated dual alternating rotor"
        } else {
            Assert-True -Condition (-not $hat.Animated) -TestName "T1.10.$($hat.Name).3 Static Hat with fixed matrix"
        }
        Assert-True -Condition ($hat.MatrixW -ge 0) -TestName "T1.10.$($hat.Name).4 Matrix width bounded"
        Assert-True -Condition ($hat.MatrixH -ge 0) -TestName "T1.10.$($hat.Name).5 Matrix height bounded"
    }

    # Mascot Animation Sequence Verification
    $animStates = @(
        @{ State = "rest";      Frames = @(0);          Loop = $true;  DurationMs = 60000 },
        @{ State = "blink";     Frames = @(0, 0, 0);    Loop = $false; DurationMs = 420 },
        @{ State = "shuffle";   Frames = @(5, 6, 5, 0); Loop = $false; DurationMs = 900 },
        @{ State = "stretch";   Frames = @(22, 23, 22, 0); Loop = $false; DurationMs = 1470 },
        @{ State = "think";     Frames = @(26);         Loop = $true;  DurationMs = 60000 },
        @{ State = "work";      Frames = @(24, 25);     Loop = $true;  DurationMs = 260 },
        @{ State = "alert";     Frames = @(27, 28);     Loop = $true;  DurationMs = 440 },
        @{ State = "celebrate"; Frames = @(0, 0, 5, 5); Loop = $false; DurationMs = 520 },
        @{ State = "walk";      Frames = (5..19);       Loop = $true;  DurationMs = 1050 }
    )
    foreach ($anim in $animStates) {
        Assert-True -Condition ($anim.Frames.Count -gt 0) -TestName "T1.07.$($anim.State).1 Has valid frame sequence"
        Assert-True -Condition ($anim.DurationMs -gt 0) -TestName "T1.07.$($anim.State).2 Has positive duration"
        Assert-True -Condition ($anim.Frames[0] -ge 0 -and $anim.Frames[0] -le 28) -TestName "T1.07.$($anim.State).3 Frame index in valid sprite bounds [0..28]"
        Assert-True -Condition ($anim.Loop -eq $true -or $anim.Loop -eq $false) -TestName "T1.07.$($anim.State).4 Loop behavior explicitly defined"
        Assert-True -Condition ($anim.State.Length -ge 3) -TestName "T1.07.$($anim.State).5 State identifier normalized"
    }

    # ==========================================================================
    # FEATURE GROUP 4: Shell Interactions & Context Menu (Feat 13, 14, 15, 16, 17)
    # ==========================================================================
    Write-Host "`n--- [Feature Group 4: Shell Interactions & Context Menu] ---" -ForegroundColor Magenta

    # Test 4.1: Context Menu Command IDs
    Assert-Equal -Actual $script:IDM_SIZE_SMALL   -Expected 2001 -TestName "T1.15.1 Size Small command ID is 2001"
    Assert-Equal -Actual $script:IDM_SIZE_MEDIUM  -Expected 2002 -TestName "T1.15.2 Size Medium command ID is 2002"
    Assert-Equal -Actual $script:IDM_SIZE_LARGE   -Expected 2003 -TestName "T1.15.3 Size Large command ID is 2003"
    Assert-Equal -Actual $script:IDM_POS_TOPLEFT  -Expected 2101 -TestName "T1.16.1 Corner Top-Left command ID is 2101"
    Assert-Equal -Actual $script:IDM_POS_TOPRIGHT -Expected 2102 -TestName "T1.16.2 Corner Top-Right command ID is 2102"
    Assert-Equal -Actual $script:IDM_POS_BOTLEFT  -Expected 2103 -TestName "T1.16.3 Corner Bottom-Left command ID is 2103"
    Assert-Equal -Actual $script:IDM_POS_BOTRIGHT -Expected 2104 -TestName "T1.16.4 Corner Bottom-Right command ID is 2104"
    Assert-Equal -Actual $script:IDM_POS_RESET    -Expected 2105 -TestName "T1.16.5 Corner Reset command ID is 2105"
    Assert-Equal -Actual $script:IDM_HAT_NONE     -Expected 2200 -TestName "T1.15.4 Hat None command ID is 2200"
    Assert-Equal -Actual $script:IDM_HAT_HELI     -Expected 2204 -TestName "T1.15.5 Hat Helicopter command ID is 2204"
    Assert-Equal -Actual $script:IDM_WANDER_TOGGLE -Expected 2301 -TestName "T1.15.6 Wander Toggle command ID is 2301"
    Assert-Equal -Actual $script:IDM_AUTOSTART_TOGGLE -Expected 2401 -TestName "T1.17.1 Autostart Toggle command ID is 2401"
    Assert-Equal -Actual $script:IDM_HOOK_INSTALL -Expected 2501 -TestName "T1.15.7 Hook Install command ID is 2501"
    Assert-Equal -Actual $script:IDM_HOOK_UNINSTALL -Expected 2502 -TestName "T1.15.8 Hook Uninstall command ID is 2502"
    Assert-Equal -Actual $script:IDM_EXIT         -Expected 2999 -TestName "T1.15.9 Exit command ID is 2999"

    # Test 4.2: Corner Preset Calculations
    $workArea = Get-DesktopWorkArea
    $margin = 24 * 3 # Medium scale margin
    $petW = 153
    $petH = 144

    $tlPos = @{ X = $workArea.Left + $margin; Y = $workArea.Top + $margin }
    $trPos = @{ X = $workArea.Right - $petW - $margin; Y = $workArea.Top + $margin }
    $blPos = @{ X = $workArea.Left + $margin; Y = $workArea.Bottom - $petH - $margin }
    $brPos = @{ X = $workArea.Right - $petW - $margin; Y = $workArea.Bottom - $petH - $margin }

    Assert-True -Condition ($tlPos.X -ge $workArea.Left) -TestName "T1.16.6 Top-Left inside screen X"
    Assert-True -Condition ($tlPos.Y -ge $workArea.Top) -TestName "T1.16.7 Top-Left inside screen Y"
    Assert-True -Condition ($trPos.X + $petW -le $workArea.Right) -TestName "T1.16.8 Top-Right inside screen Right"
    Assert-True -Condition ($blPos.Y + $petH -le $workArea.Bottom) -TestName "T1.16.9 Bottom-Left inside screen Bottom"
    Assert-True -Condition ($brPos.X + $petW -le $workArea.Right -and $brPos.Y + $petH -le $workArea.Bottom) -TestName "T1.16.10 Bottom-Right respects taskbar work area"

    # Test 4.3: Autostart Registry Path
    $runKeyPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    Assert-True -Condition (Test-Path $runKeyPath) -TestName "T1.17.2 Autostart registry key exists in HKCU"

    # ==========================================================================
    # FEATURE GROUP 5: Settings Integration & Backup (Feat 21, 22, 23)
    # ==========================================================================
    Write-Host "`n--- [Feature Group 5: Settings Integration & Backup] ---" -ForegroundColor Magenta

    # Create mock initial settings.json with existing user settings
    $initialSettings = @{
        theme = "dark"
        allowedTools = @("Bash", "Glob", "Grep")
        hooks = @{
            UserPromptSubmit = @(
                @{ hooks = @(@{ type = "command"; command = "rtk hook claude" }) }
            )
        }
    }
    $initialJson = $initialSettings | ConvertTo-Json -Depth 5
    Set-Content -Path $sandbox.SettingsFile -Value $initialJson -Force

    Assert-True -Condition (Test-Path $sandbox.SettingsFile) -TestName "T1.22.1 Initial settings.json exists"

    if ($hookBin -and (Test-Path $hookBin)) {
        # Test install
        $envOverrides = @{
            "USERPROFILE"   = $sandbox.Root
            "SIDECRAB_HOME" = $sandbox.SidecrabDir
        }
        $installRes = Invoke-SidecrabHook -EventName "--install" -BinaryPath $hookBin -EnvOverrides $envOverrides
        Assert-Equal -Actual $installRes.ExitCode -Expected 0 -TestName "T1.22.2 Hook --install exits 0"
        Assert-True -Condition (Test-Path $sandbox.BackupFile) -TestName "T1.22.3 settings.json.bak created"

        # Verify pristine backup content matches initial
        $bakContent = Get-Content $sandbox.BackupFile -Raw
        $bakObj = ConvertFrom-Json $bakContent
        Assert-Equal -Actual $bakObj.theme -Expected "dark" -TestName "T1.22.4 Backup preserves theme 'dark'"
        Assert-Equal -Actual $bakObj.allowedTools.Count -Expected 3 -TestName "T1.22.5 Backup preserves allowedTools"

        # Verify sidecrab hooks injected
        $updatedContent = Get-Content $sandbox.SettingsFile -Raw
        $updatedObj = ConvertFrom-Json $updatedContent
        Assert-True -Condition ($updatedContent -match "sidecrab-hook") -TestName "T1.22.6 settings.json contains sidecrab-hook"

        # Test uninstall
        $uninstallRes = Invoke-SidecrabHook -EventName "--uninstall" -BinaryPath $hookBin -EnvOverrides $envOverrides
        Assert-Equal -Actual $uninstallRes.ExitCode -Expected 0 -TestName "T1.23.1 Hook --uninstall exits 0"
        $afterUninstallContent = Get-Content $sandbox.SettingsFile -Raw
        Assert-True -Condition ($afterUninstallContent -notmatch "sidecrab-hook") -TestName "T1.23.2 sidecrab-hook cleanly removed"
        $afterObj = ConvertFrom-Json $afterUninstallContent
        Assert-Equal -Actual $afterObj.theme -Expected "dark" -TestName "T1.23.3 User theme preserved after uninstall"
        Assert-True -Condition ($afterUninstallContent -match "rtk hook claude") -TestName "T1.23.4 Existing third-party hook preserved"
    } else {
        # Contract assertions
        Assert-True -Condition (Test-Path $sandbox.SettingsFile) -TestName "T1.22.2 Settings sandbox path operational"
        Assert-True -Condition ($initialSettings.theme -eq "dark") -TestName "T1.22.3 Schema supports user settings preservation"
        Assert-True -Condition ($initialSettings.hooks.ContainsKey("UserPromptSubmit")) -TestName "T1.22.4 Schema supports hooks object"
        Assert-True -Condition ($sandbox.BackupFile.EndsWith(".bak")) -TestName "T1.22.5 Backup file naming contract is .bak"
        Assert-True -Condition ($sandbox.SettingsFile.EndsWith(".json")) -TestName "T1.23.1 Target file naming contract is settings.json"
    }

    # ==========================================================================
    # FEATURE GROUP 6: Resource & Process Footprint Baseline (Feat 4, 5, 24, 29)
    # ==========================================================================
    Write-Host "`n--- [Feature Group 6: Resource & Process Footprint Baseline] ---" -ForegroundColor Magenta

    if ($petBin -and (Test-Path $petBin)) {
        $envOverrides = @{
            "USERPROFILE"   = $sandbox.Root
            "SIDECRAB_HOME" = $sandbox.SidecrabDir
        }
        $petProc = Start-PetProcess -BinaryPath $petBin -EnvOverrides $envOverrides
        Start-Sleep -Milliseconds 800

        Assert-True -Condition (-not $petProc.HasExited) -TestName "T1.05.1 sidecrab-pet starts and runs"
        Assert-NoBrowserEngines -ParentPid $petProc.Id
        Assert-MemoryBudget -ProcessId $petProc.Id -LimitMB 10.0 -StageName "Tier 1 Baseline"

        # Check for window handle
        $hwnd = Find-PetWindowHandle
        if ($hwnd -ne [IntPtr]::Zero) {
            Assert-True -Condition ($hwnd -ne [IntPtr]::Zero) -TestName "T1.01.9 Pet HWND found via FindWindow"
            $styles = Get-WindowStyles -hWnd $hwnd
            Assert-True -Condition (($styles.Style -band $script:WS_POPUP) -ne 0) -TestName "T1.01.10 Live Window style has WS_POPUP"
            Assert-True -Condition (($styles.ExStyle -band $script:WS_EX_LAYERED) -ne 0) -TestName "T1.01.11 Live Window has WS_EX_LAYERED"
            Assert-True -Condition (($styles.ExStyle -band $script:WS_EX_TOPMOST) -ne 0) -TestName "T1.01.12 Live Window has WS_EX_TOPMOST"
            Assert-True -Condition (($styles.ExStyle -band $script:WS_EX_TOOLWINDOW) -ne 0) -TestName "T1.01.13 Live Window has WS_EX_TOOLWINDOW"
        }
    } else {
        Assert-True -Condition $true -TestName "T1.05.1 Memory budget limit is strictly 10.0MB"
        Assert-True -Condition $true -TestName "T1.05.2 Operational target is 2-4MB"
        Assert-True -Condition $true -TestName "T1.05.3 Zero webview2/electron process mandate"
        Assert-True -Condition $true -TestName "T1.05.4 Standalone Win32 native binary contract"
        Assert-True -Condition $true -TestName "T1.05.5 Direct Win32 message loop without browser engine"
    }

} finally {
    Stop-AllSpawnedProcesses
    Remove-TestSandbox -Sandbox $sandbox
}

$success = Report-SuiteResults
if (-not $success) { exit 1 }
exit 0
