param([Parameter(Mandatory=$true)][string]$Root,[string]$BuildRoot=(Join-Path $env:LOCALAPPDATA 'OpenProjectManager-build'))
$ErrorActionPreference='Stop'
$Root=[IO.Path]::GetFullPath($Root)
$scratch=Join-Path $BuildRoot ('smoke-' + [guid]::NewGuid().ToString('N'))
$state=Join-Path $scratch 'OpenProjectManager'
$data=Join-Path $state 'postgresql-data'
$ruby=Join-Path $Root 'runtime\ruby\bin\ruby.exe'
$pgBin=Join-Path $Root 'runtime\postgresql\bin'
$web=$null
$worker=$null
$started=$false
$previousProgramData=$env:PROGRAMDATA
$previousPath=$env:PATH
function New-TestSecret {
    $bytes=New-Object byte[] 32
    $rng=[Security.Cryptography.RandomNumberGenerator]::Create()
    try{$rng.GetBytes($bytes)}finally{$rng.Dispose()}
    return ([BitConverter]::ToString($bytes)).Replace('-','').ToLowerInvariant()
}
function Run-RubyTask([string]$Task) {
    & $ruby (Join-Path $Root 'service.rb') $Task *> (Join-Path $state "$Task.log")
    if($LASTEXITCODE -ne 0){throw "Packaged $Task failed; inspect $state\$Task.log"}
}
New-Item -ItemType Directory -Force -Path (Join-Path $state 'private'),(Join-Path $state 'tmp'),(Join-Path $state 'files') | Out-Null
$identity=[Security.Principal.WindowsIdentity]::GetCurrent().Name
& icacls.exe $scratch '/inheritance:r' '/grant:r' "${identity}:(OI)(CI)F" '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' | Out-Null
if($LASTEXITCODE -ne 0){throw 'Cannot protect test credentials'}
$databasePassword=New-TestSecret
$adminPassword=New-TestSecret
$loginPassword=New-TestSecret
[IO.File]::WriteAllText((Join-Path $state 'manager.json'),(@{folder=$Root;host='127.0.0.1';port=18088}|ConvertTo-Json))
[IO.File]::WriteAllText((Join-Path $state 'private\runtime.json'),(@{DatabasePassword=$databasePassword;SecretKeyBase=New-TestSecret}|ConvertTo-Json))
$passwordFile=Join-Path $state 'init-password'
$env:PROGRAMDATA=$scratch
$env:PATH="$(Split-Path $ruby);$pgBin;$env:SystemRoot\System32;$env:SystemRoot"
try {
    $listener=New-Object Net.Sockets.TcpListener([Net.IPAddress]::Loopback,15434)
    try{$listener.Start()}finally{$listener.Stop()}
    [IO.File]::WriteAllText($passwordFile,$adminPassword)
    & (Join-Path $pgBin 'initdb.exe') -D $data -U openproject_admin --encoding=UTF8 --locale=C --locale-provider=icu --icu-locale=en-US --auth=scram-sha-256 "--pwfile=$passwordFile" *> (Join-Path $state 'initdb.log')
    if($LASTEXITCODE -ne 0){throw "Test initdb failed; see $state\initdb.log"}
    Remove-Item -LiteralPath $passwordFile
    # Keep the PostgreSQL daemon out of PowerShell's inherited output pipeline.
    $pgStart=Start-Process -FilePath (Join-Path $pgBin 'pg_ctl.exe') -ArgumentList @('-D',"`"$data`"",'-l',"`"$(Join-Path $state 'postgresql.log')`"",'-o','"-h 127.0.0.1 -p 15434"','-w','start') -WindowStyle Hidden -PassThru
    if(!$pgStart.WaitForExit(75000) -or $pgStart.ExitCode -ne 0){throw 'Test PostgreSQL did not start'}
    $started=$true
    $env:PGPASSWORD=$adminPassword
    $sql="CREATE ROLE openproject LOGIN PASSWORD '$databasePassword'; CREATE DATABASE openproject OWNER openproject;"
    $ErrorActionPreference='Continue'
    try{$sql | & (Join-Path $pgBin 'psql.exe') -X -w -h 127.0.0.1 -p 15434 -U openproject_admin -d postgres -v ON_ERROR_STOP=1 -q 2>$null | Out-Null}
    finally{$ErrorActionPreference='Stop'}
    if($LASTEXITCODE -ne 0){throw 'Test database provisioning failed'}
    Remove-Item Env:PGPASSWORD
    Run-RubyTask migrate
    $env:OPENPROJECT_SEED__ADMIN__USER__PASSWORD=$loginPassword
    $env:OPENPROJECT_SEED__ADMIN__USER__PASSWORD__RESET='false'
    $env:OPENPROJECT_SEED__ADMIN__USER__MAIL='admin@localhost.invalid'
    Run-RubyTask seed
    Remove-Item Env:OPENPROJECT_SEED__ADMIN__USER__PASSWORD
    foreach($mode in 'web','worker') {
        $child=Start-Process -FilePath $ruby -ArgumentList @("`"$(Join-Path $Root 'service.rb')`"",$mode) -WorkingDirectory (Join-Path $Root 'openproject') -WindowStyle Hidden -RedirectStandardOutput (Join-Path $state "$mode.log") -RedirectStandardError (Join-Path $state "$mode-error.log") -PassThru
        if($mode -eq 'web'){$web=$child}else{$worker=$child}
    }
    $ready=$false
    for($attempt=0;$attempt -lt 180;$attempt++) {
        if($web.HasExited){throw "Packaged web exited. See $state\web-error.log"}
        try{$response=Invoke-WebRequest 'http://127.0.0.1:18088/login' -UseBasicParsing -TimeoutSec 5 -SessionVariable session; if($response.StatusCode -eq 200){$ready=$true;break}}catch{}
        Start-Sleep -Seconds 2
    }
    if(!$ready){throw "Packaged web did not become ready. See $state\web.log"}
    $token=[regex]::Match($response.Content,'name="authenticity_token"[^>]*value="([^"]+)"').Groups[1].Value
    if(!$token){throw 'Login form is missing its CSRF token'}
    $logged=Invoke-WebRequest 'http://127.0.0.1:18088/login' -UseBasicParsing -WebSession $session -Method Post -Body @{username='admin';password=$loginPassword;authenticity_token=[Net.WebUtility]::HtmlDecode($token)} -TimeoutSec 30
    if($logged.StatusCode -ne 200 -or $logged.Content -notmatch 'OpenProject Admin'){throw 'Packaged login did not reach the authenticated home page'}
    $asset=[regex]::Match($logged.Content,'src="([^"]*assets/frontend/main-[^"]+\.js)"').Groups[1].Value
    if(!$asset){throw 'Compiled frontend entry point is missing'}
    $assetUrl=[uri]::new([uri]'http://127.0.0.1:18088',$asset)
    if((Invoke-WebRequest $assetUrl -UseBasicParsing -TimeoutSec 30).StatusCode -ne 200){throw 'Compiled frontend could not be served'}
    if($worker.HasExited){throw "Packaged worker exited. See $state\worker-error.log"}
    $env:PGPASSWORD=$databasePassword
    $completed=0
    for($attempt=0;$attempt -lt 30;$attempt++) {
        if($worker.HasExited){throw "Packaged worker exited. See $state\worker-error.log"}
        $completed=& (Join-Path $pgBin 'psql.exe') -X -w -h 127.0.0.1 -p 15434 -U openproject -d openproject -Atc 'SELECT count(*) FROM good_jobs WHERE performed_at IS NOT NULL;'
        if($LASTEXITCODE -ne 0){throw 'Cannot query background job results'}
        if([int]$completed -gt 0){break}
        Start-Sleep -Seconds 2
    }
    if([int]$completed -eq 0){throw 'Packaged worker has not processed jobs'}
    Write-Output "PASS: fresh packaged PostgreSQL, migrations, seed, authenticated login, frontend asset and background jobs. Logs: $state"
} finally {
    foreach($child in @($web,$worker)){if($child -and !$child.HasExited){$child.Kill();$child.WaitForExit()}}
    if($started){& (Join-Path $pgBin 'pg_ctl.exe') -D $data -m fast -w stop}
    Remove-Item Env:PGPASSWORD,Env:OPENPROJECT_SEED__ADMIN__USER__PASSWORD -ErrorAction SilentlyContinue
    if(Test-Path $passwordFile){Remove-Item -LiteralPath $passwordFile}
    $env:PROGRAMDATA=$previousProgramData
    $env:PATH=$previousPath
}
