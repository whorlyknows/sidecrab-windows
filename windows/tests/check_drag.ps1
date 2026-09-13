. (Join-Path $PSScriptRoot "test_harness.ps1")

$csharp = @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class WinTest {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);

    public static IntPtr FindPetWindow(uint targetPid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((hWnd, lParam) => {
            uint pid;
            GetWindowThreadProcessId(hWnd, out pid);
            if (pid == targetPid) {
                var sb = new StringBuilder(256);
                GetClassName(hWnd, sb, 256);
                if (sb.ToString() == "SidecrabPetWindow") {
                    found = hWnd;
                    return false;
                }
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
'@
Add-Type -TypeDefinition $csharp -Language CSharp

# Ensure no sidecrab is running
Get-Process -Name *sidecrab* -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

$petBin = Find-PetBinary
$sandbox = New-TestSandbox
$envMap = @{
    "USERPROFILE"   = $sandbox.Root
    "SIDECRAB_HOME" = $sandbox.SidecrabDir
}

$proc = Start-PetProcess -BinaryPath $petBin -EnvOverrides $envMap
Start-Sleep -Milliseconds 1200

try {
    $petHwnd = [WinTest]::FindPetWindow($proc.Id)
    Write-Host "Pet HWND: $petHwnd"

    # Test PostMessage or SendMessage of WM_LBUTTONDOWN
    Write-Host "Testing PostMessage WM_LBUTTONDOWN..."
    $WM_LBUTTONDOWN = 0x0201
    $WM_LBUTTONUP = 0x0202
    $postDown = [Sidecrab.NativeMethods]::PostMessageW($petHwnd, [uint32]$WM_LBUTTONDOWN, [IntPtr]1, [IntPtr]::Zero)
    Write-Host "PostMessage result: $postDown"
    Start-Sleep -Milliseconds 200

    $postUp = [Sidecrab.NativeMethods]::PostMessageW($petHwnd, [uint32]$WM_LBUTTONUP, [IntPtr]0, [IntPtr]::Zero)
    Write-Host "PostMessage Up result: $postUp"
    Start-Sleep -Milliseconds 200

    Write-Host "Testing SetWindowPos move..."
    $geo = Get-WindowGeometry -hWnd $petHwnd
    Write-Host "Position before: ($($geo.Left), $($geo.Top))"
    
    # Move window via SetWindowPos
    $csharpMove = @'
    using System;
    using System.Runtime.InteropServices;
    public class WinMove {
        [DllImport("user32.dll")]
        public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
    }
'@
    Add-Type -TypeDefinition $csharpMove -Language CSharp
    [WinMove]::SetWindowPos($petHwnd, [IntPtr]::Zero, 300, 400, $geo.Width, $geo.Height, 0x0004 -bor 0x0010)
    Start-Sleep -Milliseconds 200
    $geo2 = Get-WindowGeometry -hWnd $petHwnd
    Write-Host "Position after: ($($geo2.Left), $($geo2.Top))"

} finally {
    Stop-AllSpawnedProcesses
    Remove-TestSandbox -Sandbox $sandbox
}
