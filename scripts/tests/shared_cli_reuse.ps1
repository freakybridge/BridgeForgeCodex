param([string]$RepositoryRoot, [string]$Base)
$ErrorActionPreference='Stop'
Set-StrictMode -Version 2
Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Utility/Microsoft.PowerShell.Utility.psd1') -ErrorAction Stop
$tokens=$null; $errors=$null
$ast=[System.Management.Automation.Language.Parser]::ParseFile((Join-Path $RepositoryRoot 'scripts/bridgeforge_codex_shared_update.ps1'),[ref]$tokens,[ref]$errors)
if($errors.Count){throw ($errors|Out-String)}
foreach($f in $ast.FindAll({param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst]},$true)) {
    if($f.Name -ne 'Invoke-Main'){ . ([scriptblock]::Create($f.Extent.Text)) }
}
$script:PhaseTimings=[ordered]@{}
$script:BuildCounts=[ordered]@{cargo_build_invocations=0;installed_cli_reused=0}
$source=Join-Path $Base 'source'
$profileRoot=Join-Path $Base 'profile'
New-Item -ItemType Directory -Path (Join-Path $source 'source'),$profileRoot -Force|Out-Null
[IO.File]::WriteAllText((Join-Path $source 'VERSION'),(Get-Content -LiteralPath (Join-Path $RepositoryRoot 'VERSION') -Raw).Trim())
[IO.File]::WriteAllText((Join-Path $source 'Cargo.toml'),"[workspace]`n")
[IO.File]::WriteAllText((Join-Path $source 'Cargo.lock'),"version = 4`n")
[IO.File]::WriteAllText((Join-Path $source 'source/main.rs'),"fn main() {}`n")
$script:FixtureCli=Join-Path $RepositoryRoot '.codex/bin/bridgeforge.exe'
function cargo {
    $arguments=@($args)
    if($arguments -notcontains '--locked' -or $arguments[-2] -ne '--bin' -or $arguments[-1] -ne 'bridgeforge'){throw 'Incorrect updater build recipe'}
    if((Get-Location).Path -ne $source){throw 'Compiler cwd differs from identity probe'}
    $index=[Array]::IndexOf($arguments,'--target-dir')
    $output=Join-Path $arguments[$index+1] 'release'
    New-Item -ItemType Directory -Path $output -Force|Out-Null
    Copy-Item -LiteralPath $script:FixtureCli -Destination (Join-Path $output 'bridgeforge.exe')
    $global:LASTEXITCODE=0
}
$first=New-RustCliPlan -RepositoryRoot $source -UserProfile $profileRoot -OperationId ([Guid]::NewGuid().ToString('N'))
Install-RustCli -Plan $first
Save-RustCliProvenance -Plan $first -RepositoryRoot $source -UserProfile $profileRoot
if(-not $first.cache_identity){throw 'Expected supported environment'}
$second=New-RustCliPlan -RepositoryRoot $source -UserProfile $profileRoot -OperationId ([Guid]::NewGuid().ToString('N'))
if($second.needs_swap -or $script:BuildCounts.cargo_build_invocations -ne 1 -or $script:BuildCounts.installed_cli_reused -ne 1){throw 'Repeated updater rebuilt the same CLI'}
[IO.File]::WriteAllText((Join-Path $source 'source/main.rs'),'fn main() { println!("changed"); }')
$third=New-RustCliPlan -RepositoryRoot $source -UserProfile $profileRoot -OperationId ([Guid]::NewGuid().ToString('N'))
if($script:BuildCounts.cargo_build_invocations -ne 2){throw 'Changed source did not rebuild'}
Undo-RustCli -Plan $third
[IO.File]::WriteAllText((Join-Path $source 'source/main.rs'),"fn main() {}`n")
[IO.File]::WriteAllText((Join-Path $source 'Cargo.lock'),"version = 4`n# changed lock`n")
$fourth=New-RustCliPlan -RepositoryRoot $source -UserProfile $profileRoot -OperationId ([Guid]::NewGuid().ToString('N'))
if($script:BuildCounts.cargo_build_invocations -ne 3){throw 'Changed lock did not rebuild'}
Undo-RustCli -Plan $fourth
$provenance=Join-Path $profileRoot '.codex/bin/bridgeforge-cli-provenance.json'
[IO.File]::WriteAllText($provenance,'{"schema":99,"owner":"unknown"}')
Save-RustCliProvenance -Plan $first -RepositoryRoot $source -UserProfile $profileRoot
if([IO.File]::ReadAllText($provenance) -ne '{"schema":99,"owner":"unknown"}'){throw 'Unknown cache metadata overwritten'}
$externalSource = Join-Path $Base 'external-source'
New-Item -ItemType Directory -Path $externalSource -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $externalSource 'entry.rs'),'pub fn external() {}')
$linkedSource = Join-Path $source 'source/linked'
New-Item -ItemType Junction -Path $linkedSource -Target $externalSource | Out-Null
$buildsBeforeLinkedProbe = $script:BuildCounts.cargo_build_invocations
$linkedRejected = $false
try { New-RustCliPlan -RepositoryRoot $source -UserProfile $profileRoot -OperationId ([Guid]::NewGuid().ToString('N')) | Out-Null }
catch { if ($_.Exception.Message -notlike '*reparse*') { throw }; $linkedRejected = $true }
if (-not $linkedRejected) { throw 'Nested source junction was accepted.' }
if ($script:BuildCounts.cargo_build_invocations -ne $buildsBeforeLinkedProbe) { throw 'Linked source reached Cargo before rejection.' }
Write-Output 'CLI reuse, source invalidation, narrow recipe, unknown metadata and nested junction guards passed.'
