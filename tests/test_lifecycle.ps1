$proc = [System.Diagnostics.Process]::Start('a:\CODE\claude pet\target\release\sidecrab-pet.exe')
Write-Host "Started PID: $($proc.Id)"
for ($i = 1; $i -le 15; $i++) {
    Start-Sleep -Seconds 1
    Write-Host "t = $i s, HasExited: $($proc.HasExited)"
    if ($proc.HasExited) {
        Write-Host "ExitCode: $($proc.ExitCode)"
        break
    }
}
if (-not $proc.HasExited) {
    Write-Host "Process stayed running for 15s. Stopping now."
    $proc.Kill()
}
