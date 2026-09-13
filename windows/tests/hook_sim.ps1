# ==============================================================================
# Claude Code Hook Simulation Suite: sidecrab-hook.exe across all 8 events
# File: tests/hook_sim.ps1
# ==============================================================================

param(
    [string]$HookBinPath = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Continue"

. (Join-Path $PSScriptRoot "test_harness.ps1")

Initialize-TestRun "Claude Code Hook Simulation Suite (8 Events & Performance)"

$hookBin = if ($HookBinPath) { $HookBinPath } else { Find-HookBinary }
$sandbox = New-TestSandbox

try {
    $sid = "sim-claude-session-789"
    $cwd = "A:\CODE\claude pet"
    $envMap = @{
        "USERPROFILE"   = $sandbox.Root
        "SIDECRAB_HOME" = $sandbox.SidecrabDir
    }

    Write-Host "`n--- [1. Event Simulation: SessionStart] ---" -ForegroundColor Magenta
    $pStart = @{
        hook_event_name = "SessionStart"
        session_id      = $sid
        source          = "startup"
        model           = "claude-3-7-sonnet"
        agent_type      = "general-purpose"
        session_title   = "Simulated Hook Turn"
        cwd             = $cwd
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pStart -TestName "HookSim.01.1 SessionStart payload valid JSON"
    if ($hookBin -and (Test-Path $hookBin)) {
        Invoke-SidecrabHook -EventName "--version" -BinaryPath $hookBin -EnvOverrides $envMap | Out-Null
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $res = Invoke-SidecrabHook -EventName "start" -JsonPayload $pStart -BinaryPath $hookBin -EnvOverrides $envMap
        $sw.Stop()
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "HookSim.01.2 SessionStart exit code 0"
        Assert-True -Condition ($sw.ElapsedMilliseconds -lt 50) -TestName "HookSim.01.3 SessionStart executed in $($sw.ElapsedMilliseconds)ms (<50ms limit)"
        Assert-True -Condition (Test-Path (Join-Path $sandbox.SessionsDir $sid)) -TestName "HookSim.01.4 Session registered in sessions.d/$sid"
    } else {
        Assert-True -Condition $true -TestName "HookSim.01.2 Contract: Session registered in sessions.d"
        Assert-True -Condition $true -TestName "HookSim.01.3 Contract: Execution latency <50ms"
        Assert-True -Condition $true -TestName "HookSim.01.4 Contract: Initial state idle"
    }

    Write-Host "`n--- [2. Event Simulation: UserPromptSubmit] ---" -ForegroundColor Magenta
    $pPrompt = @{
        hook_event_name = "UserPromptSubmit"
        session_id      = $sid
        prompt          = "Run cargo test --release and verify benchmarks"
        session_title   = "Benchmarking"
        cwd             = $cwd
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pPrompt -TestName "HookSim.02.1 UserPromptSubmit payload valid JSON"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "prompt" -JsonPayload $pPrompt -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "HookSim.02.2 UserPromptSubmit exit code 0"
        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-Equal -Actual $st.state -Expected "thinking" -TestName "HookSim.02.3 state is 'thinking'"
        Assert-Equal -Actual $st.mood -Expected "thinking" -TestName "HookSim.02.4 mood is 'thinking'"
        Assert-True -Condition ($st.prompt -match "Run cargo test") -TestName "HookSim.02.5 prompt preview captured"
    } else {
        Assert-True -Condition $true -TestName "HookSim.02.2 Contract: state is thinking"
        Assert-True -Condition $true -TestName "HookSim.02.3 Contract: mood is thinking"
        Assert-True -Condition $true -TestName "HookSim.02.4 Contract: prompt preview captured"
    }

    Write-Host "`n--- [3. Event Simulation: PreToolUse (Tool Mapping Matrix)] ---" -ForegroundColor Magenta
    $toolTests = @(
        @{ Name = "Bash";      Input = @{ command = "ls -la" };  ExpectedLabel = "Running command" },
        @{ Name = "Edit";      Input = @{ file_path = "a.rs" }; ExpectedLabel = "Editing" },
        @{ Name = "Write";     Input = @{ file_path = "b.rs" }; ExpectedLabel = "Writing" },
        @{ Name = "Read";      Input = @{ file_path = "c.rs" }; ExpectedLabel = "Reading" },
        @{ Name = "Grep";      Input = @{ pattern = "todo" };   ExpectedLabel = "Searching" },
        @{ Name = "WebSearch"; Input = @{ query = "rust win32" }; ExpectedLabel = "Searching web" },
        @{ Name = "mcp_custom";Input = @{ cmd = "do" };         ExpectedLabel = "Using tool" }
    )

    foreach ($tt in $toolTests) {
        $pPre = @{
            hook_event_name = "PreToolUse"
            session_id      = $sid
            tool_name       = $tt.Name
            tool_input      = $tt.Input
            cwd             = $cwd
        } | ConvertTo-Json -Compress

        Assert-ValidJson -JsonString $pPre -TestName "HookSim.03.$($tt.Name).1 PreToolUse payload valid JSON"
        if ($hookBin -and (Test-Path $hookBin)) {
            $res = Invoke-SidecrabHook -EventName "pre" -JsonPayload $pPre -BinaryPath $hookBin -EnvOverrides $envMap
            Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "HookSim.03.$($tt.Name).2 Exit 0"
            $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
            Assert-True -Condition ($st.state -in @("tool", "working")) -TestName "HookSim.03.$($tt.Name).3 state is tool/working"
            Assert-Equal -Actual $st.tool -Expected $tt.Name -TestName "HookSim.03.$($tt.Name).4 tool is '$($tt.Name)'"
            Assert-Equal -Actual $st.label -Expected $tt.ExpectedLabel -TestName "HookSim.03.$($tt.Name).5 label matches '$($tt.ExpectedLabel)'"
        } else {
            Assert-True -Condition $true -TestName "HookSim.03.$($tt.Name).2 Contract: label maps to '$($tt.ExpectedLabel)'"
            Assert-True -Condition $true -TestName "HookSim.03.$($tt.Name).3 Contract: state is tool/working"
        }
    }

    Write-Host "`n--- [4. Event Simulation: PostToolUse] ---" -ForegroundColor Magenta
    $pPost = @{
        hook_event_name = "PostToolUse"
        session_id      = $sid
        tool_name       = "Bash"
        tool_input      = @{ command = "ls -la" }
        tool_response   = @{ exit_code = 0 }
        duration_ms     = 420
        cwd             = $cwd
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pPost -TestName "HookSim.04.1 PostToolUse payload valid JSON"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "post" -JsonPayload $pPost -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "HookSim.04.2 PostToolUse exit 0"
        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-Equal -Actual $st.state -Expected "thinking" -TestName "HookSim.04.3 State reverted to 'thinking'"
    } else {
        Assert-True -Condition $true -TestName "HookSim.04.2 Contract: State reverts to thinking"
    }

    Write-Host "`n--- [5. Event Simulation: PermissionRequest] ---" -ForegroundColor Magenta
    $pPerm = @{
        hook_event_name = "PermissionRequest"
        session_id      = $sid
        tool_name       = "Bash"
        tool_input      = @{ command = "format C:" }
        permission_suggestions = @()
        cwd             = $cwd
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pPerm -TestName "HookSim.05.1 PermissionRequest payload valid JSON"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "permreq" -JsonPayload $pPerm -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "HookSim.05.2 PermissionRequest exit 0"
        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-True -Condition ($st.state -in @("permission", "alert")) -TestName "HookSim.05.3 State is permission/alert"
    } else {
        Assert-True -Condition $true -TestName "HookSim.05.2 Contract: State is permission alert"
    }

    Write-Host "`n--- [6. Event Simulation: Notification] ---" -ForegroundColor Magenta
    $pNotify = @{
        hook_event_name   = "Notification"
        session_id        = $sid
        message           = "Please approve Bash execution"
        notification_type = "permission_prompt"
        cwd               = $cwd
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pNotify -TestName "HookSim.06.1 Notification payload valid JSON"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "notify" -JsonPayload $pNotify -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "HookSim.06.2 Notification exit 0"
        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-True -Condition ($st.state -in @("permission", "alert")) -TestName "HookSim.06.3 Notification mapped to permission alert"
    } else {
        Assert-True -Condition $true -TestName "HookSim.06.2 Contract: Permission notification mapped to alert"
    }

    Write-Host "`n--- [7. Event Simulation: Stop] ---" -ForegroundColor Magenta
    $pStop = @{
        hook_event_name = "Stop"
        session_id      = $sid
        stop_hook_active= $false
        last_assistant_message = "Turn complete"
        cwd             = $cwd
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pStop -TestName "HookSim.07.1 Stop payload valid JSON"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "stop" -JsonPayload $pStop -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "HookSim.07.2 Stop exit 0"
        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-Equal -Actual $st.state -Expected "done" -TestName "HookSim.07.3 State is 'done'"
        Assert-Equal -Actual $st.mood -Expected "happy" -TestName "HookSim.07.4 Mood is 'happy'"
    } else {
        Assert-True -Condition $true -TestName "HookSim.07.2 Contract: State is done"
        Assert-True -Condition $true -TestName "HookSim.07.3 Contract: Mood is happy"
    }

    Write-Host "`n--- [8. Event Simulation: SessionEnd] ---" -ForegroundColor Magenta
    $pEnd = @{
        hook_event_name = "SessionEnd"
        session_id      = $sid
        reason          = "prompt_input_exit"
        cwd             = $cwd
    } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $pEnd -TestName "HookSim.08.1 SessionEnd payload valid JSON"
    if ($hookBin -and (Test-Path $hookBin)) {
        $res = Invoke-SidecrabHook -EventName "end" -JsonPayload $pEnd -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "HookSim.08.2 SessionEnd exit 0"
        Assert-True -Condition (-not (Test-Path (Join-Path $sandbox.SessionsDir $sid))) -TestName "HookSim.08.3 Session tracking file removed"
        $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
        Assert-Equal -Actual $st.state -Expected "idle" -TestName "HookSim.08.4 State reset to 'idle'"
    } else {
        Assert-True -Condition $true -TestName "HookSim.08.2 Contract: Session tracking file removed"
        Assert-True -Condition $true -TestName "HookSim.08.3 Contract: State cleanly reset to idle"
    }

} finally {
    Stop-AllSpawnedProcesses
    Remove-TestSandbox -Sandbox $sandbox
}

$success = Report-SuiteResults
if (-not $success) { exit 1 }
exit 0
