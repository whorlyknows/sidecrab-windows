. (Join-Path $PSScriptRoot "test_harness.ps1")

$csharp = @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class WinInspector2 {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);

    public static List<Tuple<IntPtr, string>> FindAllWindowsForPid(uint targetPid) {
        var list = new List<Tuple<IntPtr, string>>();
        EnumWindows((hWnd, lParam) => {
            uint pid;
            GetWindowThreadProcessId(hWnd, out pid);
            if (pid == targetPid) {
                var sb = new StringBuilder(256);
                GetClassName(hWnd, sb, 256);
                list.Add(new Tuple<IntPtr, string>(hWnd, sb.ToString()));
            }
            return true;
        }, IntPtr.Zero);
        return list;
    }
}
'@
Add-Type -TypeDefinition $csharp -Language CSharp

$petBin = Find-PetBinary
$sandbox = New-TestSandbox
$envMap = @{
    "USERPROFILE"   = $sandbox.Root
    "SIDECRAB_HOME" = $sandbox.SidecrabDir
}

$proc = Start-PetProcess -BinaryPath $petBin -EnvOverrides $envMap
Start-Sleep -Milliseconds 1200

try {
    Write-Host "Pet PID: $($proc.Id) Exited: $($proc.HasExited)"
    $wins = [WinInspector2]::FindAllWindowsForPid($proc.Id)
    foreach ($w in $wins) {
        Write-Host "HWND: $($w.Item1) Class: '$($w.Item2)'"
    }

    # Specifically find SidecrabPetWindow
    $petHwnd = [IntPtr]::Zero
    foreach ($w in $wins) {
        if ($w.Item2 -eq "SidecrabPetWindow") {
            $petHwnd = $w.Item1
            break
        }
    }

    if ($petHwnd -ne [IntPtr]::Zero) {
        Write-Host "SUCCESS: Found SidecrabPetWindow HWND: $petHwnd"
        $geo = Get-WindowGeometry -hWnd $petHwnd
        Write-Host "Initial geometry: $($geo.Width) x $($geo.Height) at ($($geo.Left), $($geo.Top))"

        # Test NCHITTEST at transparent pixel (Left + 0, Top + 0)
        $hit0 = Test-WindowHitTest -hWnd $petHwnd -ScreenX $geo.Left -ScreenY $geo.Top
        Write-Host "HitTest at (0,0) transparent: $hit0 (Expected -1, HTTRANSPARENT)"

        # Test NCHITTEST at mascot body (Left + Width/2, Top + Height * 2/3)
        $cx = $geo.Left + [int]($geo.Width / 2)
        $cy = $geo.Top + [int]($geo.Height * 2 / 3)
        $hitBody = Test-WindowHitTest -hWnd $petHwnd -ScreenX $cx -ScreenY $cy
        Write-Host "HitTest at mascot body ($cx, $cy): $hitBody (Expected 1, HTCLIENT)"

        # Test WM_COMMAND with IDM_SIZE_L = 103
        $WM_COMMAND = 0x0111
        $resCmd = [Sidecrab.NativeMethods]::SendMessageW($petHwnd, [uint32]$WM_COMMAND, [IntPtr]103, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 300
        $geoAfterL = Get-WindowGeometry -hWnd $petHwnd
        Write-Host "Geometry after WM_COMMAND 103 (Large): $($geoAfterL.Width) x $($geoAfterL.Height)"

        # Test WM_COMMAND with IDM_SIZE_S = 101
        $resCmd = [Sidecrab.NativeMethods]::SendMessageW($petHwnd, [uint32]$WM_COMMAND, [IntPtr]101, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 300
        $geoAfterS = Get-WindowGeometry -hWnd $petHwnd
        Write-Host "Geometry after WM_COMMAND 101 (Small): $($geoAfterS.Width) x $($geoAfterS.Height)"
    } else {
        Write-Host "ERROR: SidecrabPetWindow not found!"
    }
} finally {
    Stop-AllSpawnedProcesses
    Remove-TestSandbox -Sandbox $sandbox
}
