param([string]$Source = 'D:\Quant\StratusAgent', [string]$Root = '', [string]$StashCommit='')
$ErrorActionPreference = 'Stop'
if (-not $Root) { $Root = Join-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) '../.runtime/tests/git-sync-perf' }
$Root = [IO.Path]::GetFullPath($Root)
$toolDirectory = if (Test-Path -LiteralPath (Join-Path $Source 'source/factory/src/dogfood.json')) { '.build/bridgeforge/bin' } else { '.deps/bridgeforge/bin' }
function Git([string]$At, [string[]]$Arguments) {
    $out = & git.exe --no-optional-locks -C $At @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Git failed: $Arguments" }
    $out
}
New-Item -ItemType Directory -Force -Path $Root | Out-Null
$head = if($StashCommit){Git $Source @('rev-parse',"$StashCommit^1")}else{Git $Source @('rev-parse','HEAD')}
$seed = Join-Path $Root 'seed.git'
$sample = Join-Path $Root 'sample'
if (Test-Path $seed) { throw 'Snapshot already exists; preserve it' }
& git.exe clone --bare --no-hardlinks -- $Source $seed
if ($LASTEXITCODE -ne 0) { throw 'Object snapshot failed' }
Git $seed @('remote','remove','origin')
Git $seed @('branch','perf',$head)
& git.exe clone --branch perf --no-hardlinks -- $seed $sample
if ($LASTEXITCODE -ne 0) { throw 'Sample checkout failed' }
Git $sample @('config','user.name','Forge Performance Fixture')
Git $sample @('config','user.email','fixture@example.invalid')
Git $sample @('config','core.hooksPath','.githooks')
if($StashCommit) {
    Git $sample @('read-tree','--reset','-u',$StashCommit)
    Git $sample @('read-tree','HEAD')
    $parents=((Git $sample @('rev-list','--parents','-n','1',$StashCommit)) -join '').Split(' ')
    if($parents.Count -eq 4) {
        Git $sample @('archive','-o',(Join-Path $Root 'untracked.tar'),$parents[3])
        & tar -xf (Join-Path $Root 'untracked.tar') -C $sample
        if($LASTEXITCODE -ne 0){throw 'Cannot restore immutable untracked tree'}
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $sample $toolDirectory) | Out-Null
    Get-ChildItem -LiteralPath (Join-Path $Source $toolDirectory) -Force | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $sample $toolDirectory) -Recurse -Force }
    $receipt=[ordered]@{source=$Source;head=$head;sample=$sample;remote=$seed;tracked_tree_commit=$StashCommit;untracked_tree_commit=if($parents.Count -eq 4){$parents[3]}else{$null};runtime_assets='copied separately; preserve their build receipts'}
    [IO.File]::WriteAllText((Join-Path $Root 'snapshot.json'),($receipt|ConvertTo-Json -Depth 5),[Text.UTF8Encoding]::new($false))
    $receipt|ConvertTo-Json -Compress
    return
}
$paths = ((Git $Source @('ls-files','-z','--cached','--others','--exclude-standard')) -join "`n").Split([char]0,[StringSplitOptions]::RemoveEmptyEntries) | Sort-Object -Unique
$records = @()
foreach ($path in $paths) {
    $src = Join-Path $Source $path
    $dst = [IO.Path]::GetFullPath((Join-Path $sample $path))
    if (-not $dst.StartsWith($sample + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe sample path' }
    if (Test-Path -LiteralPath $src -PathType Leaf) {
        $before = (Get-FileHash -LiteralPath $src).Hash
        New-Item -ItemType Directory -Force -Path ([IO.Path]::GetDirectoryName($dst)) | Out-Null
        Copy-Item -LiteralPath $src -Destination $dst
        $after = (Get-FileHash -LiteralPath $dst).Hash
        if ($before -ne $after) { throw "Source changed during snapshot: $path" }
        $records += [ordered]@{path=$path;sha256=$after}
    } elseif (Test-Path -LiteralPath $dst -PathType Leaf) { Remove-Item -LiteralPath $dst }
}
# Runtime assets are copied, never rebuilt in the live source checkout.
New-Item -ItemType Directory -Force -Path (Join-Path $sample $toolDirectory) | Out-Null
Get-ChildItem -LiteralPath (Join-Path $Source $toolDirectory) -Force | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $sample $toolDirectory) -Recurse -Force }
if ((Git $Source @('rev-parse','HEAD')) -ne $head) { throw 'Live HEAD changed; snapshot must be reviewed' }
foreach ($record in $records) {
    if ((Get-FileHash -LiteralPath (Join-Path $Source $record.path)).Hash -ne $record.sha256) { throw "Live input changed: $($record.path)" }
}
$receipt = [ordered]@{source=$Source;head=$head;sample=$sample;remote=$seed;files=$records.Count;inputs=$records}
[IO.File]::WriteAllText((Join-Path $Root 'snapshot.json'),($receipt | ConvertTo-Json -Depth 5),[Text.UTF8Encoding]::new($false))
[pscustomobject]@{head=$head;sample=$sample;files=$records.Count} | ConvertTo-Json -Compress
