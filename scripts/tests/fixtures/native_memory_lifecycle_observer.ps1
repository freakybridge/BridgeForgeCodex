# Diagnostic only: observe real hooks without an early process-tree kill.
param([switch]$ConfirmAuthorizedMemorySync)
$ErrorActionPreference = 'Stop'
if (-not $ConfirmAuthorizedMemorySync) {
    throw 'This live diagnostic can synchronize native memories. Explicit authorization is required.'
}
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$stateRoot = Join-Path $env:USERPROFILE '.codex/.bridgeforge-codex/native-memory-sync'
$clock = [Diagnostics.Stopwatch]::StartNew()
function Write-Observation($kind, $data) {
    @{elapsedMs=$clock.ElapsedMilliseconds;utc=[DateTime]::UtcNow.ToString('o');kind=$kind;data=$data} |
        ConvertTo-Json -Depth 12 -Compress
}
$start = [Diagnostics.ProcessStartInfo]::new()
$start.FileName = (Get-Command codex).Source
$start.Arguments = 'app-server --stdio'
$start.WorkingDirectory = $projectRoot
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$server = [Diagnostics.Process]::new()
$server.StartInfo = $start
$previous = ''
$closed = $false
$unsubscribeAt = $null
try {
    [void]$server.Start()
    Write-Observation 'server-start' @{pid=$server.Id}
    $stderrDrain = $server.StandardError.ReadToEndAsync()
    $server.StandardInput.WriteLine('{"id":1,"method":"initialize","params":{"clientInfo":{"name":"memory_lifecycle_observer","version":"2"},"capabilities":{"experimentalApi":true}}}')
    $lineTask = $server.StandardOutput.ReadLineAsync()
    while ($clock.Elapsed.TotalSeconds -lt 180) {
        $worker = $null
        if (Test-Path "$stateRoot/worker.json") {
            $worker = Get-Content "$stateRoot/worker.json" -Raw | ConvertFrom-Json -AsHashtable
        }
        $alive = $false
        if ($worker.pid) { $alive = $null -ne (Get-Process -Id $worker.pid -ErrorAction SilentlyContinue) }
        $observed = @{worker=$worker;alive=$alive;pending=(Test-Path "$stateRoot/pending.json");serverExited=$server.HasExited}
        $fingerprint = $observed | ConvertTo-Json -Depth 8 -Compress
        if ($fingerprint -ne $previous) { Write-Observation 'process-state' $observed; $previous = $fingerprint }
        if ($lineTask -and $lineTask.IsCompleted) {
            $line = $lineTask.GetAwaiter().GetResult()
            if ($null -eq $line) { Write-Observation 'stdout-eof' @{}; $lineTask = $null }
            else {
                $response = $line | ConvertFrom-Json -AsHashtable
                if ($response.error) { Write-Observation 'rpc-error' $response.error; throw 'App server RPC failed' }
                if ($response.id -eq 1) {
                    $server.StandardInput.WriteLine('{"method":"initialized","params":{}}')
                    $request = @{id=2;method='thread/start';params=@{cwd=$projectRoot;ephemeral=$true;sessionStartSource='startup'}}
                    $server.StandardInput.WriteLine(($request | ConvertTo-Json -Depth 8 -Compress))
                }
                if ($response.id -eq 2) {
                    $testThread = $response.result.thread.id
                    Write-Observation 'thread-start' @{threadId=$testThread;ephemeral=$response.result.thread.ephemeral}
                    $request = @{id=3;method='turn/start';params=@{threadId=$testThread;input=@(@{type='text';text='Lifecycle integration smoke test only. Reply OK. Do not call any tools.'})}}
                    $server.StandardInput.WriteLine(($request | ConvertTo-Json -Depth 8 -Compress))
                }
                if ($response.method -match '^hook/') { Write-Observation 'hook-event' $response }
                if ($response.method -eq 'turn/completed') {
                    Write-Observation 'turn-completed' @{status=$response.params.turn.status}
                    $request = @{id=4;method='thread/unsubscribe';params=@{threadId=$testThread}}
                    $server.StandardInput.WriteLine(($request | ConvertTo-Json -Compress))
                    Write-Observation 'unsubscribe-sent' @{}
                    $unsubscribeAt = $clock.Elapsed.TotalSeconds
                }
                if ($response.id -eq 4) { Write-Observation 'unsubscribe-result' $response.result }
                $lineTask = $server.StandardOutput.ReadLineAsync()
            }
        }
        if (-not $closed -and $null -ne $unsubscribeAt -and $clock.Elapsed.TotalSeconds -gt ($unsubscribeAt + 65)) {
            $server.StandardInput.Close()
            $closed = $true
            Write-Observation 'stdin-closed' @{}
        }
        if ($closed -and $server.HasExited -and -not $alive -and $null -eq $lineTask) { break }
        Start-Sleep -Milliseconds 100
    }
    if (-not $closed -or -not $server.HasExited -or (Test-Path "$stateRoot/pending.json") -or (Test-Path "$stateRoot/worker.json")) {
        throw 'Lifecycle verification incomplete: server or memory queue did not finish.'
    }
    $health = Get-Content "$stateRoot/health.json" -Raw | ConvertFrom-Json -AsHashtable
    Write-Observation 'final-health' $health
    if ($health.status -ne 'healthy') { throw 'Memory synchronization is not healthy.' }
} finally {
    if (-not $server.HasExited) {
        $server.StandardInput.Close()
        Write-Observation 'observation-deadline' @{forcedKill=$false;pid=$server.Id}
    } else { Write-Observation 'server-exit' @{exitCode=$server.ExitCode} }
    $server.Dispose()
}
