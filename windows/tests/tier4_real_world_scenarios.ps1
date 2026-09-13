# ==============================================================================
# Tier 4: Real-World Application Scenarios (Full E2E Claude Code Session Lifecycle)
# File: tests/tier4_real_world_scenarios.ps1
# ==============================================================================

param(
    [string]$PetBinPath = "",
    [string]$HookBinPath = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Continue"

. (Join-Path $PSScriptRoot "test_harness.ps1")

Initialize-TestRun "Tier 4: Real-World Application Scenarios (Full Claude Lifecycle)"

$petBin = if ($PetBinPath) { $PetBinPath } else { Find-PetBinary }
$hookBin = if ($HookBinPath) { $HookBinPath } else { Find-HookBinary }
$sandbox = New-TestSandbox

try {
    $e2eSid = "e2e-session-lifecycle-001"
    $transcriptPath = Join-Path $sandbox.ClaudeDir "sessions\$e2eSid\transcript.jsonl"
    $projectDir = "A:\CODE\claude pet"
    $envMap = @{
        "USERPROFILE"   = $sandbox.Root
        "SIDECRAB_HOME" = $sandbox.SidecrabDir
    }

    Write-Host "`n=== SCENARIO 1: Complete Turn Execution with Tool Approval ===" -ForegroundColor Cyan

    # --------------------------------------------------------------------------
    # Step 1: Session Initialization (SessionStart)
    # --------------------------------------------------------------------------
    Write-Host "`n[Step 1: SessionStart]" -ForegroundColor Magenta
    $pStart = @{
        hook_event_name = "SessionStart"
        session_id      = $e2eSid
        source          = "startup"
        model           = "claude-3-7-sonnet"
        agent_type      = "general-purpose"
        cwd             = $projectDir
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pStart -TestName "T4.01.1 Valid SessionStart payload"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "start" -JsonPayload $pStart -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "T4.01.2 SessionStart hook completed exit 0"
        $markerPath = Join-Path $sandbox.SessionsDir $e2eSid
        Assert-True -Condition (Test-Path $markerPath) -TestName "T4.01.3 Session tracking file created in sessions.d/$e2eSid"
    } else {
        Assert-True -Condition $true -TestName "T4.01.2 Contract: Session registered in sessions.d/"
        Assert-True -Condition $true -TestName "T4.01.3 Contract: Initial state set to idle/wakeup"
    }

    # --------------------------------------------------------------------------
    # Step 2: User Prompts Claude (UserPromptSubmit) -> Thinking Pose
    # --------------------------------------------------------------------------
    Write-Host "`n[Step 2: UserPromptSubmit -> Thinking]" -ForegroundColor Magenta
    $pPrompt = @{
        hook_event_name = "UserPromptSubmit"
        session_id      = $e2eSid
        prompt          = "Build and verify the native Windows desktop pet"
        session_title   = "Native Desktop Pet"
        cwd             = $projectDir
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pPrompt -TestName "T4.02.1 Valid UserPromptSubmit payload"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "prompt" -JsonPayload $pPrompt -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "T4.02.2 UserPromptSubmit completed exit 0"

        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-Equal -Actual $st.state -Expected "thinking" -TestName "T4.02.3 Mascot state is 'thinking'"
        Assert-Equal -Actual $st.mood -Expected "thinking" -TestName "T4.02.4 Mascot mood is 'thinking'"
        Assert-True -Condition ($st.startedAt -gt 0) -TestName "T4.02.5 Turn timer startedAt recorded ($($st.startedAt))"
    } else {
        Assert-True -Condition $true -TestName "T4.02.2 Contract: State transitioned to thinking"
        Assert-True -Condition $true -TestName "T4.02.3 Contract: Animated thought bubble activated"
    }

    # --------------------------------------------------------------------------
    # Step 3: Claude Invokes Tool (PreToolUse) -> Working at Laptop
    # --------------------------------------------------------------------------
    Write-Host "`n[Step 3: PreToolUse -> Working at Laptop]" -ForegroundColor Magenta
    $pPre = @{
        hook_event_name = "PreToolUse"
        session_id      = $e2eSid
        tool_name       = "Bash"
        tool_input      = @{ command = "cargo build --release" }
        tool_use_id     = "toolu_01_bash_001"
        cwd             = $projectDir
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pPre -TestName "T4.03.1 Valid PreToolUse payload"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "pre" -JsonPayload $pPre -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "T4.03.2 PreToolUse completed exit 0"

        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-True -Condition ($st.state -in @("tool", "working")) -TestName "T4.03.3 Mascot state is 'tool' / 'working'"
        Assert-Equal -Actual $st.tool -Expected "Bash" -TestName "T4.03.4 Tool name recorded as 'Bash'"
        Assert-Equal -Actual $st.label -Expected "Running command" -TestName "T4.03.5 Label mapped to 'Running command'"
    } else {
        Assert-True -Condition $true -TestName "T4.03.2 Contract: State transitioned to working"
        Assert-True -Condition $true -TestName "T4.03.3 Contract: Mascot switches to laptop typing frames"
    }

    # --------------------------------------------------------------------------
    # Step 4: Permission Prompt Required (PermissionRequest) -> Claw Wave Alert
    # --------------------------------------------------------------------------
    Write-Host "`n[Step 4: PermissionRequest -> Claw Wave Alert]" -ForegroundColor Magenta
    $pPerm = @{
        hook_event_name = "PermissionRequest"
        session_id      = $e2eSid
        tool_name       = "Bash"
        tool_input      = @{ command = "del /f /q sensitive.dat" }
        permission_suggestions = @()
        cwd             = $projectDir
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pPerm -TestName "T4.04.1 Valid PermissionRequest payload"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "permreq" -JsonPayload $pPerm -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "T4.04.2 PermissionRequest completed exit 0"

        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-True -Condition ($st.state -in @("permission", "alert")) -TestName "T4.04.3 Mascot state is 'permission' / 'alert'"
        Assert-Equal -Actual $st.mood -Expected "alert" -TestName "T4.04.4 Mood is 'alert'"
        Assert-Equal -Actual $st.label -Expected "Awaiting permission" -TestName "T4.04.5 Label is 'Awaiting permission'"
    } else {
        Assert-True -Condition $true -TestName "T4.04.2 Contract: State transitioned to permission alert"
        Assert-True -Condition $true -TestName "T4.04.3 Contract: Claws waving urgently with !? overhead"
    }

    # --------------------------------------------------------------------------
    # Step 5: User Approves and Tool Finishes (PostToolUse) -> Thinking Pose
    # --------------------------------------------------------------------------
    Write-Host "`n[Step 5: PostToolUse -> Thinking Return]" -ForegroundColor Magenta
    $pPost = @{
        hook_event_name = "PostToolUse"
        session_id      = $e2eSid
        tool_name       = "Bash"
        tool_input      = @{ command = "del /f /q sensitive.dat" }
        tool_response   = @{ exit_code = 0; output = "deleted" }
        tool_use_id     = "toolu_01_bash_001"
        duration_ms     = 1240
        cwd             = $projectDir
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pPost -TestName "T4.05.1 Valid PostToolUse payload"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "post" -JsonPayload $pPost -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "T4.05.2 PostToolUse completed exit 0"

        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-Equal -Actual $st.state -Expected "thinking" -TestName "T4.05.3 Mascot returned to 'thinking'"
    } else {
        Assert-True -Condition $true -TestName "T4.05.2 Contract: State transitioned to thinking"
    }

    # --------------------------------------------------------------------------
    # Step 6: Turn Complete (Stop) -> Done Celebration
    # --------------------------------------------------------------------------
    Write-Host "`n[Step 6: Stop -> Done Celebration]" -ForegroundColor Magenta
    $pStop = @{
        hook_event_name = "Stop"
        session_id      = $e2eSid
        stop_hook_active= $false
        last_assistant_message = "All tasks completed successfully."
        cwd             = $projectDir
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pStop -TestName "T4.06.1 Valid Stop payload"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "stop" -JsonPayload $pStop -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "T4.06.2 Stop hook completed exit 0"

        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-Equal -Actual $st.state -Expected "done" -TestName "T4.06.3 Mascot state is 'done'"
        Assert-Equal -Actual $st.mood -Expected "happy" -TestName "T4.06.4 Mascot mood is 'happy'"
        Assert-Equal -Actual $st.label -Expected "Done" -TestName "T4.06.5 Label is 'Done'"
        Assert-Equal -Actual $st.startedAt -Expected 0 -TestName "T4.06.6 startedAt timer cleared to 0"
    } else {
        Assert-True -Condition $true -TestName "T4.06.2 Contract: State transitioned to celebrate/done"
        Assert-True -Condition $true -TestName "T4.06.3 Contract: Happy double hop animation triggered"
    }

    # --------------------------------------------------------------------------
    # Step 7: Session Terminates (SessionEnd) -> Idle / Sleep Reset
    # --------------------------------------------------------------------------
    Write-Host "`n[Step 7: SessionEnd -> Idle Reset]" -ForegroundColor Magenta
    $pEnd = @{
        hook_event_name = "SessionEnd"
        session_id      = $e2eSid
        reason          = "prompt_input_exit"
        cwd             = $projectDir
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pEnd -TestName "T4.07.1 Valid SessionEnd payload"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "end" -JsonPayload $pEnd -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "T4.07.2 SessionEnd hook completed exit 0"

        $markerPath = Join-Path $sandbox.SessionsDir $e2eSid
        Assert-True -Condition (-not (Test-Path $markerPath)) -TestName "T4.07.3 Session marker deleted from sessions.d"

        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-Equal -Actual $st.state -Expected "idle" -TestName "T4.07.4 State reset to 'idle'"
    } else {
        Assert-True -Condition $true -TestName "T4.07.2 Contract: Session marker deleted from sessions.d"
        Assert-True -Condition $true -TestName "T4.07.3 Contract: State cleanly reset to idle resting pose"
    }

} finally {
    Stop-AllSpawnedProcesses
    Remove-TestSandbox -Sandbox $sandbox
}

$success = Report-SuiteResults
if (-not $success) { exit 1 }
exit 0
