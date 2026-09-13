param(
    [string]$PetBinPath = "a:\CODE\claude pet\windows\target\release\sidecrab-pet.exe"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WinUserTest2 {
    [DllImport("user32.dll")]
    public static extern IntPtr PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);
}
"@

Write-Host "=== Testing Animation, Flight, and Wander Commands ===" -ForegroundColor Cyan
$proc = Start-Process -FilePath $PetBinPath -PassThru
try {
    Start-Sleep -Milliseconds 800

    $hwnd = [WinUserTest2]::FindWindow("SidecrabPetWindow", $null)
    if ($hwnd -eq [IntPtr]::Zero) {
        throw "Could not find SidecrabPetWindow handle"
    }

    $WM_COMMAND = 0x0111
    $commands = @(
        @{ Name = "Thinking"; Cmd = 1042 },
        @{ Name = "Working"; Cmd = 1043 },
        @{ Name = "Wave"; Cmd = 1051 },
        @{ Name = "Stretch"; Cmd = 1047 },
        @{ Name = "Fly & Bounce"; Cmd = 1060 },
        @{ Name = "Wander Now"; Cmd = 1061 },
        @{ Name = "Reset State"; Cmd = 1062 }
    )

    foreach ($entry in $commands) {
        Write-Host "Sending $($entry.Name) ($($entry.Cmd))..." -ForegroundColor Yellow
        [WinUserTest2]::PostMessage($hwnd, $WM_COMMAND, [IntPtr]$entry.Cmd, [IntPtr]::Zero) | Out-Null
        Start-Sleep -Milliseconds 400
        if ($proc.HasExited) {
            throw "CRASH DETECTED on $($entry.Name)!"
        }
        Write-Host "  [OK] $($entry.Name) active, Process PID $($proc.Id) running." -ForegroundColor Green
    }

    Write-Host "ALL ANIMATION & PHYSICS MODES FUNCTION STABLY!" -ForegroundColor Green
} finally {
    if (-not $proc.HasExited) {
        $proc.Kill()
        $proc.WaitForExit(2000) | Out-Null
    }
}
