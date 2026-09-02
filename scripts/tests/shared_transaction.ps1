param([string]$RepositoryRoot, [string]$Base)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Utility/Microsoft.PowerShell.Utility.psd1') -ErrorAction Stop
$tokens = $null
$errors = $null
$source = Join-Path $RepositoryRoot 'scripts/bridgeforge_codex_shared_update.ps1'
$ast = [System.Management.Automation.Language.Parser]::ParseFile($source, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw ($errors | Out-String) }
foreach ($function in $ast.FindAll({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] }, $true)) {
    if ($function.Name -ne 'Invoke-Main') { . ([scriptblock]::Create($function.Extent.Text)) }
}
$CommandHomeName = '.bridgeforge-codex'
$CommandHomeLogName = '.bridgeforge-codex-home-update.json'
$TestCrashAfterActionCount = 0
$TestFailAfterSwap = ''

function Assert-True {
    param([bool]$Value, [string]$Message)
    if (-not $Value) { throw $Message }
}

foreach ($scenario in @('rollback', 'committed-cleanup')) {
    $fixtureProfile = Join-Path $Base $scenario
    $repo = Join-Path $fixtureProfile 'source'
    $bin = Join-Path $fixtureProfile '.codex/bin'
    $skills = Join-Path $fixtureProfile '.codex/skills'
    New-Item -ItemType Directory -Path $repo, $bin, $skills -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $repo 'SKILL.md'), 'new skill')
    $op = [Guid]::NewGuid().ToString('N')
    $commit = 'a' * 40
    $bundleHome = [ordered]@{
        target = Join-Path $fixtureProfile $CommandHomeName
        stage = Join-Path $fixtureProfile ".bridgeforge-codex-stage-$op"
        backup = Join-Path $fixtureProfile ".bridgeforge-codex-backup-$op"
        log = Join-Path $fixtureProfile $CommandHomeLogName
        operation_id = $op
        had_original = $true
        needs_swap = $true
        status = 'staged'
    }
    New-Item -ItemType Directory -Path $bundleHome.target, $bundleHome.stage -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $bundleHome.target 'VERSION'), 'old home')
    [IO.File]::WriteAllText((Join-Path $bundleHome.stage 'VERSION'), 'new home')
    $cli = [ordered]@{
        target = Join-Path $bin 'bridgeforge.exe'
        stage = Join-Path $bin ".bridgeforge-stage-$op.exe"
        backup = Join-Path $bin ".bridgeforge-backup-$op.exe"
        had_original = $true
        needs_swap = $true
        status = 'staged'
    }
    Copy-Item -LiteralPath (Join-Path $RepositoryRoot '.codex/bin/bridgeforge-hook.exe') -Destination $cli.target
    [IO.File]::WriteAllText($cli.stage, 'new cli transaction bytes')
    $oldCliHash = Get-Sha256 -Path $cli.target
    $fileHash = Get-Sha256 -Path (Join-Path $repo 'SKILL.md')
    $manifest = @{ platforms = @{ codex = @{ skills = @(@{ name = 'probe'; files = @(@{ source = 'SKILL.md'; target = 'SKILL.md'; sha256 = $fileHash }) }) } } } | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $plans = @(New-UpdatePlan -Manifest $manifest -UserProfile $fixtureProfile -OperationId $op -Commit $commit)
    $log = Join-Path $fixtureProfile '.bridgeforge-codex-shared-update.json'
    $script:CleanupPending = $false
    $child = $null
    try {
        if ($scenario -eq 'committed-cleanup') {
            $info = New-Object Diagnostics.ProcessStartInfo
            $info.FileName = $cli.target
            $info.Arguments = 'pre-tool'
            $info.UseShellExecute = $false
            $info.CreateNoWindow = $true
            $info.RedirectStandardInput = $true
            $info.RedirectStandardOutput = $true
            $info.RedirectStandardError = $true
            $child = New-Object Diagnostics.Process
            $child.StartInfo = $info
            Assert-True ($child.Start()) 'Cannot launch image holder'
            Start-Sleep -Milliseconds 150
            Assert-True (-not $child.HasExited) 'Image holder unexpectedly exited'
        }
        $TestFailAfterSwap = if ($scenario -eq 'rollback') { 'codex:1' } else { '' }
        $failed = $false
        try {
            Invoke-UpdateTransaction -RepositoryRoot $repo -Manifest $manifest -Commit $commit -ManifestHash ('b' * 64) -UserProfile $fixtureProfile -LogPath $log -OperationId $op -PlatformPlans $plans -CommandHomePlan $bundleHome -RustCliPlan $cli
        }
        catch {
            if ($scenario -ne 'rollback' -or $_.Exception.Message -notlike '*Injected test failure*') { throw }
            $failed = $true
        }
        if ($scenario -eq 'rollback') {
            Assert-True $failed 'Expected rollback injection'
            Assert-True ((Get-Sha256 -Path $cli.target) -eq $oldCliHash) 'CLI was not rolled back'
            Assert-True ([IO.File]::ReadAllText((Join-Path $bundleHome.target 'VERSION')) -eq 'old home') 'Home was not rolled back'
            Assert-True (-not (Test-Path (Join-Path $skills 'probe'))) 'Skill was not rolled back'
            Assert-True (-not (Test-Path $log)) 'Successful rollback left a journal'
        }
        else {
            Assert-True $script:CleanupPending 'Running image should defer backup cleanup'
            Assert-True ((Read-JsonFile -Path $log).committed) 'Shared durable commit is missing'
            Assert-True ([IO.File]::ReadAllText($cli.target) -eq 'new cli transaction bytes') 'Committed CLI was reverted'
            Assert-True ([IO.File]::ReadAllText((Join-Path $bundleHome.target 'VERSION')) -eq 'new home') 'Committed Home was reverted'
            Assert-True ([IO.File]::ReadAllText((Join-Path $skills 'probe/SKILL.md')) -eq 'new skill') 'Committed Skill was reverted'
            $child.Kill()
            $child.WaitForExit()
            Restore-InterruptedOperation -LogPath $log -UserProfile $fixtureProfile
            Assert-True (-not (Test-Path $log)) 'Deferred cleanup did not recover'
            Assert-True (-not (Test-Path $cli.backup)) 'CLI backup was not cleaned after process exit'
            Assert-True ([IO.File]::ReadAllText($cli.target) -eq 'new cli transaction bytes') 'Cleanup changed committed CLI'
        }
    }
    finally {
        if ($null -ne $child) {
            if (-not $child.HasExited) { $child.Kill(); $child.WaitForExit() }
            $child.Dispose()
        }
    }
}
Write-Output 'shared bundle rollback and deferred committed cleanup passed'
