param([Parameter(Mandatory=$true)][string]$Root)
$ErrorActionPreference='Stop'
$installer=Get-Content (Join-Path $PSScriptRoot '..\installer\openproject-manager.iss') -Raw
if($installer -notmatch 'Excludes: "\\setup-native\.ps1,\\service\.rb,') {throw 'Launcher exclusions must be rooted so gem service.rb files remain bundled'}
$setup=Join-Path $PSScriptRoot 'setup-native.ps1'
$parseErrors=$null
$tokens=$null
$ast=[Management.Automation.Language.Parser]::ParseFile($setup,[ref]$tokens,[ref]$parseErrors)
if($parseErrors){throw ($parseErrors|Out-String)}
$definition=$ast.Find({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Get-ServiceXml'},$true)
if(!$definition){throw 'Missing service XML generator'}
Invoke-Expression $definition.Extent.Text
$Root='C:\Program Files\OpenProject & Manager'
$state='C:\ProgramData\OpenProject & Manager'
foreach($mode in 'worker','web') {
    [xml]$xml=Get-ServiceXml "OpenProject$mode" $mode
    if($xml.service.executable -ne "$Root\runtime\ruby\bin\ruby.exe"){throw 'XML did not preserve the installation path'}
    if($xml.service.arguments -ne "`"$Root\service.rb`" $mode"){throw 'Service arguments are not quoted'}
    if($xml.service.depend -ne 'OpenProjectPostgreSQL'){throw 'Missing PostgreSQL dependency'}
    if($xml.service.serviceaccount.user -ne 'LocalService'){throw 'Unexpected service identity'}
    if($xml.OuterXml -match 'DatabasePassword|SecretKeyBase|<password>'){throw 'Service XML must not contain credentials'}
}
Write-Output 'PASS: escaped service paths, quoted arguments, PostgreSQL dependency and least-privilege identity'

foreach($name in 'Assert-TrustedPath','Protect-Directory','Protect-File') {
    $function=$ast.Find({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name},$true)
    if(!$function){throw "Missing ACL helper: $name"}
    Invoke-Expression $function.Extent.Text
}
function Set-Acl { param($LiteralPath,$AclObject) $script:CapturedAcl=$AclObject }
$fixture=Join-Path $env:TEMP ('OpenProjectManager-acl-' + [guid]::NewGuid().ToString('N'))
Protect-Directory $fixture
if($CapturedAcl.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne 'S-1-5-32-544'){throw 'Unexpected state owner'}
$file=Join-Path $fixture 'runtime.json'
[IO.File]::WriteAllText($file,'test-only')
Protect-File $file -ServiceRead
$rules=$CapturedAcl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])
$service=@($rules | Where-Object {$_.IdentityReference.Value -eq 'S-1-5-19'})
if($rules.Count -ne 3 -or $service.Count -ne 1 -or ($service[0].FileSystemRights -band [Security.AccessControl.FileSystemRights]::Write)){throw 'Application credentials must be read-only to LocalService'}
Protect-File $file
if(@($CapturedAcl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier]) | Where-Object {$_.IdentityReference.Value -in 'S-1-5-19','S-1-5-20'}).Count){throw 'Database administrator credentials must not be readable by the service accounts'}
$target=Join-Path $fixture 'target'
$link=Join-Path $fixture 'redirected'
New-Item -ItemType Directory -Path $target | Out-Null
New-Item -ItemType Junction -Path $link -Target $target | Out-Null
Remove-Item -LiteralPath $target
$rejected=$false
try { Assert-TrustedPath $link } catch { if($_.Exception.Message -notlike 'Refusing redirected state path:*'){throw}; $rejected=$true }
if(!$rejected){throw 'A dangling state junction was accepted'}
[IO.Directory]::Delete($link)
Remove-Item -LiteralPath $file
Remove-Item -LiteralPath $fixture
Write-Output 'PASS: administrator ownership and separate application/database credential ACLs'
