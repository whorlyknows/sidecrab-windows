param(
    [string]$PetBinPath = "a:\CODE\claude pet\windows\target\release\sidecrab-pet.exe"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WinUserTest {
    [DllImport("user32.dll")]
    public static extern IntPtr PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);
}
"@

Write-Host "=== Testing Size Change Stability ===" -ForegroundColor Cyan
$proc = Start-Process -FilePath $PetBinPath -PassThru
try {
    Start-Sleep -Milliseconds 800

    $hwnd = [WinUserTest]::FindWindow("SidecrabPetWindow", $null)
    if ($hwnd -eq [IntPtr]::Zero) {
        throw "Could not find SidecrabPetWindow handle"
    }

    $WM_COMMAND = 0x0111
    $sizes = @(1001, 1002, 1003, 1002, 1001, 1003) # S, M, L, M, S, L

    foreach ($cmd in $sizes) {
        Write-Host "Sending size change command $cmd..." -ForegroundColor Yellow
        [WinUserTest]::PostMessage($hwnd, $WM_COMMAND, [IntPtr]$cmd, [IntPtr]::Zero) | Out-Null
        Start-Sleep -Milliseconds 300
        if ($proc.HasExited) {
            throw "CRASH DETECTED! Process died after size change command $cmd!"
        }
        Write-Host "Process PID $($proc.Id) alive and healthy." -ForegroundColor Green
    }

    Write-Host "ALL SIZE CHANGES COMPLETED WITH ZERO CRASHES!" -ForegroundColor Green
} finally {
    if (-not $proc.HasExited) {
        $proc.Kill()
        $proc.WaitForExit(2000) | Out-Null
    }
}
