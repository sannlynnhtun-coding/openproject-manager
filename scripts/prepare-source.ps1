param([string]$SourceRoot=(Join-Path $env:LOCALAPPDATA 'OpenProjectManager-build\source'))
$ErrorActionPreference='Stop'
$SourceRoot=[IO.Path]::GetFullPath($SourceRoot)
$revision='36d730dca55d092336b5f6d806e9b2a56eecd70c'
$repository='https://github.com/opf/openproject.git'
function Git-Checked([string[]]$Arguments) {
    & git @Arguments
    if($LASTEXITCODE -ne 0){throw "Git failed: $($Arguments[0])"}
}
if(Test-Path -LiteralPath $SourceRoot){throw 'Choose a new source directory; existing files are never replaced.'}
New-Item -ItemType Directory -Path $SourceRoot | Out-Null
Git-Checked @('init',$SourceRoot)
Git-Checked @('-C',$SourceRoot,'remote','add','origin',$repository)
Git-Checked @('-C',$SourceRoot,'fetch','--depth=1','origin',$revision)
Git-Checked @('-C',$SourceRoot,'checkout','--detach','FETCH_HEAD')
$patches=Get-ChildItem (Join-Path $PSScriptRoot '..\patches') -Filter '*.patch' | Sort-Object Name
if($patches.Count -ne 3){throw 'Expected all three Windows port patches.'}
foreach($patch in $patches){
    Git-Checked @('-C',$SourceRoot,'apply','--check',$patch.FullName)
    Git-Checked @('-C',$SourceRoot,'apply',$patch.FullName)
}
Write-Output "Prepared native OpenProject source: $SourceRoot"
