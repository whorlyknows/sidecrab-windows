# ==============================================================================
# Claude Code Desktop Pet — Core Test Harness & Opaque-Box Assertion Library
# File: tests/test_harness.ps1
# ==============================================================================

Set-StrictMode -Version Latest
$ErrorActionPreference = "Continue"

# ------------------------------------------------------------------------------
# 1. Win32 P/Invoke Interop Declarations
# ------------------------------------------------------------------------------
if (-not ([System.Management.Automation.PSTypeName]'Sidecrab.NativeMethods').Type) {
    $csharpCode = @"
    using System;
    using System.Runtime.InteropServices;

    namespace Sidecrab {
        public struct RECT {
            public int Left;
            public int Top;
            public int Right;
            public int Bottom;
            public int Width { get { return Right - Left; } }
            public int Height { get { return Bottom - Top; } }
        }

        public struct POINT {
            public int X;
            public int Y;
        }

        public static class NativeMethods {
            [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
            public static extern IntPtr FindWindowExW(IntPtr hWndParent, IntPtr hWndChildAfter, string lpszClass, string lpszWindow);

            [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
            public static extern IntPtr FindWindowW(string lpClassName, string lpWindowName);

            [DllImport("user32.dll", SetLastError = true)]
            public static extern int GetWindowLongW(IntPtr hWnd, int nIndex);

            [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", SetLastError = true)]
            private static extern IntPtr GetWindowLongPtr64(IntPtr hWnd, int nIndex);

            public static IntPtr GetWindowLongPtr(IntPtr hWnd, int nIndex) {
                if (IntPtr.Size == 8) {
                    return GetWindowLongPtr64(hWnd, nIndex);
                } else {
                    return new IntPtr(GetWindowLongW(hWnd, nIndex));
                }
            }

            [DllImport("user32.dll", SetLastError = true)]
            [return: MarshalAs(UnmanagedType.Bool)]
            public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

            [DllImport("user32.dll", SetLastError = true)]
            [return: MarshalAs(UnmanagedType.Bool)]
            public static extern bool IsWindowVisible(IntPtr hWnd);

            [DllImport("user32.dll", SetLastError = true)]
            public static extern IntPtr SendMessageW(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

            [DllImport("user32.dll", SetLastError = true)]
            [return: MarshalAs(UnmanagedType.Bool)]
            public static extern bool PostMessageW(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

            [DllImport("user32.dll")]
            public static extern IntPtr GetForegroundWindow();

            [DllImport("user32.dll")]
            [return: MarshalAs(UnmanagedType.Bool)]
            public static extern bool SetForegroundWindow(IntPtr hWnd);

            [DllImport("user32.dll", SetLastError = true)]
            public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);

            [DllImport("user32.dll", SetLastError = true)]
            [return: MarshalAs(UnmanagedType.Bool)]
            public static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, [MarshalAs(UnmanagedType.Bool)] bool fAttach);

            [DllImport("kernel32.dll")]
            public static extern uint GetCurrentThreadId();
        }
    }
"@
    Add-Type -TypeDefinition $csharpCode -Language CSharp
}

# ------------------------------------------------------------------------------
# 2. Win32 Style & Message Constants
# ------------------------------------------------------------------------------
$script:GWL_STYLE       = -16
$script:GWL_EXSTYLE     = -20

$script:WS_POPUP        = 0x80000000L
$script:WS_VISIBLE      = 0x10000000L

$script:WS_EX_LAYERED   = 0x00080000L
$script:WS_EX_TOPMOST   = 0x00000008L
$script:WS_EX_TOOLWINDOW= 0x00000080L

$script:WM_NCHITTEST    = 0x0084
$script:HTTRANSPARENT   = -1
$script:HTCLIENT        = 1
$script:HTCAPTION       = 2

$script:WM_LBUTTONDOWN  = 0x0201
$script:WM_LBUTTONUP    = 0x0202
$script:WM_LBUTTONDBLCLK= 0x0203
$script:WM_RBUTTONUP    = 0x0205
$script:WM_USER         = 0x0400
$script:WM_SIDECRAB_STATE_CHANGED = 0x0465 # WM_USER + 101

# Context Menu Command IDs
$script:IDM_SIZE_SMALL  = 2001
$script:IDM_SIZE_MEDIUM = 2002
$script:IDM_SIZE_LARGE  = 2003
$script:IDM_POS_TOPLEFT = 2101
$script:IDM_POS_TOPRIGHT= 2102
$script:IDM_POS_BOTLEFT = 2103
$script:IDM_POS_BOTRIGHT= 2104
$script:IDM_POS_RESET   = 2105
$script:IDM_HAT_NONE    = 2200
$script:IDM_HAT_TOP     = 2201
$script:IDM_HAT_CHEF    = 2202
$script:IDM_HAT_FEDORA  = 2203
$script:IDM_HAT_HELI    = 2204
$script:IDM_WANDER_TOGGLE = 2301
$script:IDM_AUTOSTART_TOGGLE = 2401
$script:IDM_HOOK_INSTALL = 2501
$script:IDM_HOOK_UNINSTALL = 2502
$script:IDM_EXIT        = 2999

# ------------------------------------------------------------------------------
# 3. Test Runner & Assertion State
# ------------------------------------------------------------------------------
$script:CurrentSuite    = ""
$script:PassCount       = 0
$script:FailCount       = 0
$script:SkipCount       = 0
$script:TestResults     = @()
$script:SpawnedProcesses= @()

function Initialize-TestRun {
    param([string]$SuiteName)
    $script:CurrentSuite = $SuiteName
    $script:PassCount    = 0
    $script:FailCount    = 0
    $script:SkipCount    = 0
    $script:TestResults  = @()
    $script:SpawnedProcesses = @()
    Write-Host "`n==================================================================" -ForegroundColor Cyan
    Write-Host " RUNNING SUITE: $SuiteName" -ForegroundColor Cyan
    Write-Host "==================================================================" -ForegroundColor Cyan
}

# Add Windows Forms assembly for Screen WorkArea queries
try {
    Add-Type -AssemblyName System.Windows.Forms -ErrorAction SilentlyContinue
} catch {}

function Get-DesktopWorkArea {
    if ('System.Windows.Forms.Screen' -as [type]) {
        $wa = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
        return [PSCustomObject]@{
            Left   = $wa.Left
            Top    = $wa.Top
            Right  = $wa.Right
            Bottom = $wa.Bottom
            Width  = $wa.Width
            Height = $wa.Height
        }
    }
    return [PSCustomObject]@{
        Left = 0; Top = 0; Right = 1920; Bottom = 1040; Width = 1920; Height = 1040
    }
}

function Assert-True {
    param(
        [bool]$Condition,
        [string]$TestName,
        [string]$FailureMessage = "Condition was false"
    )
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    if ($Condition) {
        $stopwatch.Stop()
        $script:PassCount++
        $script:TestResults += [PSCustomObject]@{
            Suite    = $script:CurrentSuite
            Test     = $TestName
            Status   = "PASS"
            Duration = "$($stopwatch.ElapsedMilliseconds)ms"
            Message  = ""
        }
        Write-Host "  [PASS] $TestName" -ForegroundColor Green
    } else {
        $stopwatch.Stop()
        $script:FailCount++
        $script:TestResults += [PSCustomObject]@{
            Suite    = $script:CurrentSuite
            Test     = $TestName
            Status   = "FAIL"
            Duration = "$($stopwatch.ElapsedMilliseconds)ms"
            Message  = $FailureMessage
        }
        Write-Host "  [FAIL] $TestName" -ForegroundColor Red
        Write-Host "         Reason: $FailureMessage" -ForegroundColor Yellow
    }
}

function Assert-Equal {
    param(
        $Actual,
        $Expected,
        [string]$TestName
    )
    $cond = ($Actual -eq $Expected)
    $msg = "Expected: [$Expected], Actual: [$Actual]"
    Assert-True -Condition $cond -TestName $TestName -FailureMessage $msg
}

function Assert-Match {
    param(
        [string]$Actual,
        [string]$RegexPattern,
        [string]$TestName
    )
    $cond = ($Actual -match $RegexPattern)
    $msg = "Value '$Actual' did not match pattern '$RegexPattern'"
    Assert-True -Condition $cond -TestName $TestName -FailureMessage $msg
}

function Assert-LessThanOrEqual {
    param(
        [double]$Actual,
        [double]$Limit,
        [string]$TestName
    )
    $cond = ($Actual -le $Limit)
    $msg = "Value $Actual exceeded limit of $Limit"
    Assert-True -Condition $cond -TestName $TestName -FailureMessage $msg
}

function Assert-ValidJson {
    param(
        [string]$JsonString,
        [string]$TestName
    )
    try {
        $null = ConvertFrom-Json $JsonString -ErrorAction Stop
        Assert-True -Condition $true -TestName $TestName
    } catch {
        Assert-True -Condition $false -TestName $TestName -FailureMessage "Invalid JSON: $($_.Exception.Message)"
    }
}

function Report-SuiteResults {
    Write-Host "`n------------------------------------------------------------------" -ForegroundColor Cyan
    Write-Host " SUITE SUMMARY: $script:CurrentSuite" -ForegroundColor Cyan
    Write-Host " Passed: $script:PassCount | Failed: $script:FailCount | Skipped: $script:SkipCount" -ForegroundColor $(if ($script:FailCount -gt 0) { "Red" } else { "Green" })
    Write-Host "------------------------------------------------------------------`n" -ForegroundColor Cyan
    return ($script:FailCount -eq 0)
}

# ------------------------------------------------------------------------------
# 4. Binary Discovery & Execution Helpers
# ------------------------------------------------------------------------------
function Find-PetBinary {
    $candidates = @(
        $env:SIDECRAB_PET_BIN,
        "a:\CODE\claude pet\windows\target\release\sidecrab-pet.exe",
        "a:\CODE\claude pet\windows\target\release\sidecrab.exe",
        "a:\CODE\claude pet\windows\target\debug\sidecrab-pet.exe",
        "a:\CODE\claude pet\windows\target\debug\sidecrab.exe",
        "a:\CODE\claude pet\target\release\sidecrab-pet.exe",
        "a:\CODE\claude pet\target\release\sidecrab.exe",
        "a:\CODE\claude pet\target\debug\sidecrab-pet.exe",
        "a:\CODE\claude pet\target\debug\sidecrab.exe",
        "target\release\sidecrab-pet.exe",
        "target\debug\sidecrab-pet.exe"
    )
    foreach ($path in $candidates) {
        if ($path -and (Test-Path $path)) {
            return (Resolve-Path $path).Path
        }
    }
    return $null
}

function Find-HookBinary {
    $candidates = @(
        $env:SIDECRAB_HOOK_BIN,
        "a:\CODE\claude pet\windows\target\release\sidecrab-hook.exe",
        "a:\CODE\claude pet\windows\target\debug\sidecrab-hook.exe",
        "a:\CODE\claude pet\target\release\sidecrab-hook.exe",
        "a:\CODE\claude pet\target\debug\sidecrab-hook.exe",
        "target\release\sidecrab-hook.exe",
        "target\debug\sidecrab-hook.exe"
    )
    foreach ($path in $candidates) {
        if ($path -and (Test-Path $path)) {
            return (Resolve-Path $path).Path
        }
    }
    return $null
}

function Start-PetProcess {
    param(
        [string]$BinaryPath = "",
        [hashtable]$EnvOverrides = @{}
    )
    if (-not $BinaryPath) {
        $BinaryPath = Find-PetBinary
    }
    if (-not $BinaryPath -or -not (Test-Path $BinaryPath)) {
        throw "Pet binary not found. Please build release or debug target first."
    }

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $BinaryPath
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true

    foreach ($key in $EnvOverrides.Keys) {
        $psi.EnvironmentVariables[$key] = $EnvOverrides[$key]
    }

    $proc = [System.Diagnostics.Process]::Start($psi)
    $script:SpawnedProcesses += $proc
    Start-Sleep -Milliseconds 600
    return $proc
}

function Stop-AllSpawnedProcesses {
    foreach ($proc in $script:SpawnedProcesses) {
        try {
            if ($proc -and -not $proc.HasExited) {
                $proc.Kill()
                [void]$proc.WaitForExit(1000)
            }
        } catch {
            # Process may have already terminated
        }
    }
    $script:SpawnedProcesses = @()
}

function Invoke-SidecrabHook {
    param(
        [string]$EventName,
        [string]$JsonPayload = "{}",
        [string]$BinaryPath = "",
        [hashtable]$EnvOverrides = @{}
    )
    if (-not $BinaryPath) {
        $BinaryPath = Find-HookBinary
    }
    if (-not $BinaryPath -or -not (Test-Path $BinaryPath)) {
        throw "Hook binary not found."
    }

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $BinaryPath
    $psi.Arguments = $EventName
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true

    foreach ($key in $EnvOverrides.Keys) {
        $psi.EnvironmentVariables[$key] = $EnvOverrides[$key]
    }

    $proc = [System.Diagnostics.Process]::Start($psi)
    if ($null -ne $JsonPayload) {
        $sw = $proc.StandardInput
        $sw.Write($JsonPayload)
        $sw.Close()
    } else {
        $proc.StandardInput.Close()
    }

    $stdoutTask = $proc.StandardOutput.ReadToEndAsync()
    $stderrTask = $proc.StandardError.ReadToEndAsync()

    $exited = $proc.WaitForExit(5000)
    if (-not $exited) {
        try {
            $proc.Kill()
            [void]$proc.WaitForExit(1000)
        } catch {}
    }

    $stdout = if ($stdoutTask.Wait(1000)) { $stdoutTask.Result } else { "" }
    $stderr = if ($stderrTask.Wait(1000)) { $stderrTask.Result } else { "" }

    return [PSCustomObject]@{
        ExitCode = $proc.ExitCode
        StdOut   = $stdout
        StdErr   = $stderr
    }
}

# ------------------------------------------------------------------------------
# 5. Opaque-Box Memory & Process Inspection
# ------------------------------------------------------------------------------
function Get-ProcessMemoryMB {
    param([int]$ProcessId)
    $proc = Get-Process -Id $ProcessId -ErrorAction Stop
    $wsMB = [Math]::Round($proc.WorkingSet64 / 1MB, 3)
    $privMB = [Math]::Round($proc.PrivateMemorySize64 / 1MB, 3)
    return [PSCustomObject]@{
        WorkingSetMB   = $wsMB
        PrivateMemoryMB= $privMB
        WorkingSetRaw  = $proc.WorkingSet64
        PrivateRaw     = $proc.PrivateMemorySize64
    }
}

function Assert-MemoryBudget {
    param(
        [int]$ProcessId,
        [double]$LimitMB = 10.0,
        [string]$StageName = "Operation"
    )
    $mem = Get-ProcessMemoryMB -ProcessId $ProcessId
    $msg = "$StageName WorkingSet64: $($mem.WorkingSetMB) MB (Limit: $LimitMB MB)"
    return (Assert-LessThanOrEqual -Actual $mem.WorkingSetMB -Limit $LimitMB -TestName "Memory Ceiling <10MB ($StageName)")
}

function Assert-NoBrowserEngines {
    param([int]$ParentPid)
    $children = Get-CimInstance Win32_Process -Filter "ParentProcessId = $ParentPid" -ErrorAction SilentlyContinue
    $forbidden = @("msedgewebview2.exe", "electron.exe", "node.exe", "chrome.exe", "cef.exe")
    $foundForbidden = @()
    if ($children) {
        foreach ($c in $children) {
            if ($forbidden -contains $c.Name.ToLower()) {
                $foundForbidden += $c.Name
            }
        }
    }
    $cond = ($foundForbidden.Count -eq 0)
    $msg = "Detected forbidden browser engine processes: $($foundForbidden -join ', ')"
    return (Assert-True -Condition $cond -TestName "Zero Browser Engines Spawned" -FailureMessage $msg)
}

# ------------------------------------------------------------------------------
# 6. Win32 Window Verification Helpers
# ------------------------------------------------------------------------------
function Find-PetWindowHandle {
    param(
        [string]$ClassName = "ClaudePetWindowClass",
        [string]$WindowTitle = "Claude Code Pet"
    )
    $hwnd = [Sidecrab.NativeMethods]::FindWindowExW([IntPtr]::Zero, [IntPtr]::Zero, $ClassName, $null)
    if ($hwnd -eq [IntPtr]::Zero) {
        $hwnd = [Sidecrab.NativeMethods]::FindWindowW($ClassName, $null)
    }
    if ($hwnd -eq [IntPtr]::Zero) {
        $hwnd = [Sidecrab.NativeMethods]::FindWindowW($null, $WindowTitle)
    }
    return $hwnd
}

function Get-WindowStyles {
    param([IntPtr]$hWnd)
    $style = [Sidecrab.NativeMethods]::GetWindowLongPtr($hWnd, $script:GWL_STYLE).ToInt64()
    $exStyle = [Sidecrab.NativeMethods]::GetWindowLongPtr($hWnd, $script:GWL_EXSTYLE).ToInt64()
    return [PSCustomObject]@{
        Style   = $style
        ExStyle = $exStyle
    }
}

function Get-WindowGeometry {
    param([IntPtr]$hWnd)
    $rect = New-Object Sidecrab.RECT
    $ok = [Sidecrab.NativeMethods]::GetWindowRect($hWnd, [ref]$rect)
    if (-not $ok) { return $null }
    return [PSCustomObject]@{
        Left   = $rect.Left
        Top    = $rect.Top
        Right  = $rect.Right
        Bottom = $rect.Bottom
        Width  = $rect.Width
        Height = $rect.Height
    }
}

function Test-WindowHitTest {
    param(
        [IntPtr]$hWnd,
        [int]$ScreenX,
        [int]$ScreenY
    )
    $lParam = [IntPtr]::new(($ScreenY -shl 16) -bor ($ScreenX -band 0xFFFF))
    $res = [Sidecrab.NativeMethods]::SendMessageW($hWnd, [uint32]$script:WM_NCHITTEST, [IntPtr]::Zero, $lParam)
    return $res.ToInt32()
}

# ------------------------------------------------------------------------------
# 7. Sandbox & Safe Settings Isolation
# ------------------------------------------------------------------------------
function New-TestSandbox {
    $guid = [Guid]::NewGuid().ToString("N").Substring(0, 8)
    $sandboxPath = Join-Path $env:TEMP "sidecrab_sandbox_$guid"
    $sidecrabDir = Join-Path $sandboxPath ".sidecrab"
    $claudeDir   = Join-Path $sandboxPath ".claude"
    $sessionsDir = Join-Path $sidecrabDir "sessions.d"

    New-Item -ItemType Directory -Path $sessionsDir -Force | Out-Null
    New-Item -ItemType Directory -Path $claudeDir -Force | Out-Null

    return [PSCustomObject]@{
        Root        = $sandboxPath
        SidecrabDir = $sidecrabDir
        ClaudeDir   = $claudeDir
        SessionsDir = $sessionsDir
        StateFile   = Join-Path $sidecrabDir "state.json"
        ConfigFile  = Join-Path $sidecrabDir "config.json"
        SettingsFile= Join-Path $claudeDir "settings.json"
        BackupFile  = Join-Path $claudeDir "settings.json.bak"
    }
}

function Remove-TestSandbox {
    param([PSCustomObject]$Sandbox)
    if ($Sandbox -and (Test-Path $Sandbox.Root)) {
        try {
            Remove-Item -Path $Sandbox.Root -Recurse -Force -ErrorAction SilentlyContinue
        } catch {}
    }
}
