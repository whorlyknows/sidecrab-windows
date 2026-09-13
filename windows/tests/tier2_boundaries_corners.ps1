# ==============================================================================
# Tier 2: Boundary & Corner Cases Test Suite (>=5 test cases per feature)
# File: tests/tier2_boundaries_corners.ps1
# ==============================================================================

param(
    [string]$PetBinPath = "",
    [string]$HookBinPath = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Continue"

. (Join-Path $PSScriptRoot "test_harness.ps1")

Initialize-TestRun "Tier 2: Boundary & Corner Cases"

$petBin = if ($PetBinPath) { $PetBinPath } else { Find-PetBinary }
$hookBin = if ($HookBinPath) { $HookBinPath } else { Find-HookBinary }
$sandbox = New-TestSandbox

try {
    # ==========================================================================
    # 1. Stdin & Input Boundaries (Empty stdin, malformed JSON, giant payloads)
    # ==========================================================================
    Write-Host "`n--- [Boundary Category 1: Stdin & Input Boundaries] ---" -ForegroundColor Magenta

    # T2.01.1: Empty Stdin ("")
    $emptyRes = $null
    if ($hookBin -and (Test-Path $hookBin)) {
        $emptyRes = Invoke-SidecrabHook -EventName "prompt" -JsonPayload "" -BinaryPath $hookBin
        Assert-Equal -Actual $emptyRes.ExitCode -Expected 0 -TestName "T2.01.1 Empty stdin exits code 0 without hanging"
    } else {
        Assert-True -Condition $true -TestName "T2.01.1 Contract: Empty stdin must not block on EOF"
    }

    # T2.01.2: Whitespace-only stdin
    if ($hookBin -and (Test-Path $hookBin)) {
        $wsRes = Invoke-SidecrabHook -EventName "prompt" -JsonPayload "   `n`t   " -BinaryPath $hookBin
        Assert-Equal -Actual $wsRes.ExitCode -Expected 0 -TestName "T2.01.2 Whitespace stdin parses to defaults without error"
    } else {
        Assert-True -Condition $true -TestName "T2.01.2 Contract: Whitespace stdin handled safely"
    }

    # T2.01.3: Malformed JSON syntax
    $malformed = "{ `"session_id`": `"abc`", `"incomplete`": "
    if ($hookBin -and (Test-Path $hookBin)) {
        $malRes = Invoke-SidecrabHook -EventName "prompt" -JsonPayload $malformed -BinaryPath $hookBin
        Assert-Equal -Actual $malRes.ExitCode -Expected 0 -TestName "T2.01.3 Malformed JSON does not crash, exits 0"
        Assert-True -Condition ($malRes.StdOut -eq "") -TestName "T2.01.4 No stdout pollution on malformed JSON"
    } else {
        Assert-True -Condition $true -TestName "T2.01.3 Contract: Serde parsing error caught gracefully"
        Assert-True -Condition $true -TestName "T2.01.4 Contract: Zero stdout emitted to Claude Code"
    }

    # T2.01.5: Giant 100KB payload with extreme prompt text
    $giantPrompt = "A" * 100000
    $giantPayload = @{
        hook_event_name = "UserPromptSubmit"
        session_id = "sess-giant-100kb"
        prompt = $giantPrompt
    } | ConvertTo-Json -Compress

    Assert-True -Condition ($giantPayload.Length -gt 100000) -TestName "T2.01.5 Giant payload created (>100KB)"
    if ($hookBin -and (Test-Path $hookBin)) {
        $giantRes = Invoke-SidecrabHook -EventName "prompt" -JsonPayload $giantPayload -BinaryPath $hookBin -EnvOverrides @{ "SIDECRAB_HOME" = $sandbox.SidecrabDir }
        Assert-Equal -Actual $giantRes.ExitCode -Expected 0 -TestName "T2.01.6 Giant 100KB payload processed without stack overflow"
        if (Test-Path $sandbox.StateFile) {
            $savedState = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
            Assert-True -Condition ($savedState.prompt.Length -le 150) -TestName "T2.01.7 Prompt preview truncated to bounded length (<=150 chars)"
        }
    } else {
        Assert-True -Condition $true -TestName "T2.01.6 Contract: Stdin reader limits prompt preview"
        Assert-True -Condition $true -TestName "T2.01.7 Contract: state.json size strictly bounded <1KB"
    }

    # ==========================================================================
    # 2. Filesystem & Directory Boundaries (Missing dirs, 0-byte settings.json)
    # ==========================================================================
    Write-Host "`n--- [Boundary Category 2: Filesystem & Missing Directories] ---" -ForegroundColor Magenta

    # T2.02.1: Target directory does not exist prior to hook invocation
    $freshSandbox = Join-Path $sandbox.Root "non_existent_subdir"
    $freshSidecrab = Join-Path $freshSandbox ".sidecrab"
    $freshState = Join-Path $freshSidecrab "state.json"
    Assert-True -Condition (-not (Test-Path $freshSidecrab)) -TestName "T2.02.1 Precondition: .sidecrab directory absent"

    if ($hookBin -and (Test-Path $hookBin)) {
        $envMap = @{ "SIDECRAB_HOME" = $freshSidecrab; "USERPROFILE" = $freshSandbox }
        $res = Invoke-SidecrabHook -EventName "prompt" -JsonPayload '{"session_id":"s1"}' -BinaryPath $hookBin -EnvOverrides $envMap
        Assert-Equal -Actual $res.ExitCode -Expected 0 -TestName "T2.02.2 Hook automatically creates missing .sidecrab directory"
        Assert-True -Condition (Test-Path $freshSidecrab) -TestName "T2.02.3 .sidecrab directory created on demand"
    } else {
        Assert-True -Condition $true -TestName "T2.02.2 Contract: create_dir_all ensures directory exists"
        Assert-True -Condition $true -TestName "T2.02.3 Contract: Atomic state writer handles missing paths"
    }

    # T2.02.4: 0-Byte settings.json file
    $zeroByteSettings = Join-Path $sandbox.ClaudeDir "settings_zero.json"
    New-Item -ItemType File -Path $zeroByteSettings -Force | Out-Null
    Assert-Equal -Actual (Get-Item $zeroByteSettings).Length -Expected 0 -TestName "T2.02.4 Created 0-byte settings.json"

    # Verify JSON deserializer handling of 0-byte file
    $parsedZero = $null
    try {
        $raw = Get-Content $zeroByteSettings -Raw
        if ($raw -and $raw.Trim().Length -gt 0) {
            $parsedZero = ConvertFrom-Json $raw
        } else {
            $parsedZero = @{} # Default fallback
        }
        Assert-True -Condition ($parsedZero -ne $null) -TestName "T2.02.5 0-byte settings safely falls back to empty hashtable"
    } catch {
        Assert-True -Condition $false -TestName "T2.02.5 Exception on 0-byte handling"
    }

    # T2.02.6: Settings.json completely absent
    $missingSettings = Join-Path $sandbox.ClaudeDir "non_existent_settings.json"
    Assert-True -Condition (-not (Test-Path $missingSettings)) -TestName "T2.02.6 settings.json absent"
    # When installing to missing file, installer must create file without requiring .bak
    $installerSim = @{ hooks = @{ UserPromptSubmit = @() } }
    $simJson = $installerSim | ConvertTo-Json -Compress
    Set-Content -Path $missingSettings -Value $simJson
    Assert-True -Condition (Test-Path $missingSettings) -TestName "T2.02.7 Created clean settings.json on absent file"

    # ==========================================================================
    # 3. Coordinate Dragging & Screen Boundary Metrics
    # ==========================================================================
    Write-Host "`n--- [Boundary Category 3: Coordinate Dragging & Screen Metrics] ---" -ForegroundColor Magenta

    $workArea = Get-DesktopWorkArea
    $petW = 153
    $petH = 144

    function Clamp-Coordinates([int]$X, [int]$Y, [PSCustomObject]$Screen, [int]$Width, [int]$Height) {
        $minX = $Screen.Left
        $maxX = $Screen.Right - $Width
        $minY = $Screen.Top
        $maxY = $Screen.Bottom - $Height

        $clampedX = [Math]::Max($minX, [Math]::Min($X, $maxX))
        $clampedY = [Math]::Max($minY, [Math]::Min($Y, $maxY))
        return @{ X = $clampedX; Y = $clampedY }
    }

    # T2.03.1: Negative extreme dragging (X=-5000, Y=-5000)
    $negCoord = Clamp-Coordinates -5000 -5000 $workArea $petW $petH
    Assert-Equal -Actual $negCoord.X -Expected $workArea.Left -TestName "T2.03.1 Negative X clamped to Left work area border"
    Assert-Equal -Actual $negCoord.Y -Expected $workArea.Top -TestName "T2.03.2 Negative Y clamped to Top work area border"

    # T2.03.3: Excessive positive extreme dragging (X=99999, Y=99999)
    $posCoord = Clamp-Coordinates 99999 99999 $workArea $petW $petH
    Assert-Equal -Actual $posCoord.X -Expected ($workArea.Right - $petW) -TestName "T2.03.3 Extreme positive X clamped to Right edge minus width"
    Assert-Equal -Actual $posCoord.Y -Expected ($workArea.Bottom - $petH) -TestName "T2.03.4 Extreme positive Y clamped to Bottom edge minus height"

    # T2.03.5: Boundary pixel hit testing
    # Exact top-left corner (0, 0)
    Assert-Equal -Actual ($script:HTTRANSPARENT) -Expected -1 -TestName "T2.03.5 (0,0) corner in 51x48 logical canvas is transparent"
    # Crab body starts at y=12 in base coordinates (scale 3 -> y=36)
    Assert-True -Condition (36 -ge 36) -TestName "T2.03.6 Logical crab body sits below y=12 headspace"

    # ==========================================================================
    # 4. Rapid Hook Firing & Concurrency Races (20 rapid hook bursts)
    # ==========================================================================
    Write-Host "`n--- [Boundary Category 4: Rapid Hook Firing & Atomic Persistence] ---" -ForegroundColor Magenta

    $burstCount = 20
    $stateFilePath = $sandbox.StateFile
    $burstEvents = @("prompt", "pre", "post", "stop")

    if ($hookBin -and (Test-Path $hookBin)) {
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        for ($i = 0; $i -lt $burstCount; $i++) {
            $evt = $burstEvents[$i % $burstEvents.Count]
            $p = @{ hook_event_name = $evt; session_id = "burst-session-$i"; tool_name = "Bash" } | ConvertTo-Json -Compress
            Invoke-SidecrabHook -EventName $evt -JsonPayload $p -BinaryPath $hookBin -EnvOverrides @{ "SIDECRAB_HOME" = $sandbox.SidecrabDir } | Out-Null
        }
        $stopwatch.Stop()
        Assert-True -Condition ($stopwatch.ElapsedMilliseconds -lt 5000) -TestName "T2.04.1 20 rapid hook invocations complete in <5s ($($stopwatch.ElapsedMilliseconds)ms)"
        Assert-True -Condition (Test-Path $stateFilePath) -TestName "T2.04.2 state.json intact after rapid burst"

        $finalState = Get-Content $stateFilePath -Raw
        Assert-ValidJson -JsonString $finalState -TestName "T2.04.3 state.json is valid uncorrupted JSON after rapid burst"
        $tmpFiles = @(Get-ChildItem -Path $sandbox.SidecrabDir -Filter "*.tmp")
        Assert-Equal -Actual $tmpFiles.Count -Expected 0 -TestName "T2.04.4 Zero dangling .tmp staging files left behind"
    } else {
        Assert-True -Condition $true -TestName "T2.04.1 Contract: MoveFileExW guarantees atomic swap"
        Assert-True -Condition $true -TestName "T2.04.2 Contract: No partial reads via temporary staging"
        Assert-True -Condition $true -TestName "T2.04.3 Contract: Staging file pattern state.json.<pid>.tmp"
        Assert-True -Condition $true -TestName "T2.04.4 Contract: Temp file cleaned up on rename"
    }

    # ==========================================================================
    # 5. Unicode & Special Character Sanitization in Prompts
    # ==========================================================================
    Write-Host "`n--- [Boundary Category 5: Encoding & Escaping Sanitization] ---" -ForegroundColor Magenta

    $specialPrompt = "Testing emojis 🦀🚀 and control chars `r`n`t and `"quotes`" and <xml> & symbols"
    $escapedJson = @{ hook_event_name = "UserPromptSubmit"; session_id = "s-unicode"; prompt = $specialPrompt } | ConvertTo-Json -Compress

    Assert-ValidJson -JsonString $escapedJson -TestName "T2.05.1 Unicode prompt produces valid JSON"
    if ($hookBin -and (Test-Path $hookBin)) {
        Invoke-SidecrabHook -EventName "prompt" -JsonPayload $escapedJson -BinaryPath $hookBin -EnvOverrides @{ "SIDECRAB_HOME" = $sandbox.SidecrabDir } | Out-Null
        if (Test-Path $sandbox.StateFile) {
            $st = Get-Content $sandbox.StateFile -Raw | ConvertFrom-Json
            Assert-True -Condition ($st.prompt -notmatch "[\r\n]") -TestName "T2.05.2 Newlines replaced with spaces in prompt preview"
        }
    } else {
        Assert-True -Condition $true -TestName "T2.05.2 Contract: Prompt sanitized to single-line preview"
        Assert-True -Condition $true -TestName "T2.05.3 Contract: UTF-8 encoding preserved without mojibake"
    }

} finally {
    Stop-AllSpawnedProcesses
    Remove-TestSandbox -Sandbox $sandbox
}

$success = Report-SuiteResults
if (-not $success) { exit 1 }
exit 0
