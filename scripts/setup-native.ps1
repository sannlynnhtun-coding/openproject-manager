param([Parameter(Mandatory=$true)][string]$Root, [switch]$CheckBundle)
$ErrorActionPreference = 'Stop'
$Root = [IO.Path]::GetFullPath($Root)
$ruby = Join-Path $Root 'runtime\ruby\bin\ruby.exe'
$pgBin = Join-Path $Root 'runtime\postgresql\bin'
$app = Join-Path $Root 'openproject'
$state = Join-Path $env:PROGRAMDATA 'OpenProjectManager'

function Invoke-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$([IO.Path]::GetFileName($Program)) failed (exit $LASTEXITCODE)." }
}
function Assert-TrustedPath([string]$Path) {
    $item=Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
    if (!$item) { return }
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Refusing redirected state path: $Path" }
    $owner=(Get-Acl -LiteralPath $Path).GetOwner([Security.Principal.SecurityIdentifier]).Value
    $trusted=@('S-1-5-18','S-1-5-32-544',[Security.Principal.WindowsIdentity]::GetCurrent().User.Value)
    if($trusted -notcontains $owner){throw "Existing state has an unexpected owner: $Path"}
}
function Protect-Directory([string]$Path, [string]$ServiceSid) {
    Assert-TrustedPath $Path
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
    $acl=New-Object Security.AccessControl.DirectorySecurity
    $acl.SetAccessRuleProtection($true,$false)
    $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
    $identities=@('S-1-5-18','S-1-5-32-544')
    if($ServiceSid){$identities+=$ServiceSid}
    foreach($id in $identities){$acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule((New-Object Security.Principal.SecurityIdentifier($id)),'FullControl','ContainerInherit,ObjectInherit','None','Allow')))}
    Set-Acl -LiteralPath $Path -AclObject $acl
}
function Protect-File([string]$Path,[switch]$ServiceRead) {
    Assert-TrustedPath $Path
    $acl=New-Object Security.AccessControl.FileSecurity
    $acl.SetAccessRuleProtection($true,$false)
    $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
    foreach($id in 'S-1-5-18','S-1-5-32-544'){$acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule((New-Object Security.Principal.SecurityIdentifier($id)),'FullControl','Allow')))}
    if($ServiceRead){$acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule((New-Object Security.Principal.SecurityIdentifier('S-1-5-19')),'Read','Allow')))}
    Set-Acl -LiteralPath $Path -AclObject $acl
}
function New-Secret {
    $bytes = New-Object byte[] 32
    $rng = [Security.Cryptography.RandomNumberGenerator]::Create()
    try { $rng.GetBytes($bytes) } finally { $rng.Dispose() }
    return ([BitConverter]::ToString($bytes)).Replace('-', '').ToLowerInvariant()
}
function Invoke-RubyTask([string]$Task,[string]$Log,[switch]$Append) {
    $previousPreference=$ErrorActionPreference
    try {
        $ErrorActionPreference='Continue'
        & $ruby (Join-Path $Root 'service.rb') $Task 2>&1 | Out-File -LiteralPath $Log -Encoding utf8 -Append:$Append
    } finally { $ErrorActionPreference=$previousPreference }
    if($LASTEXITCODE -ne 0){throw "$Task failed. See $Log. Database retained."}
}
function Get-ServiceXml([string]$Name, [string]$Mode) {
    $escapedRoot = [Security.SecurityElement]::Escape($Root)
    $logPath = [Security.SecurityElement]::Escape((Join-Path $state 'logs'))
    return @"
<service>
  <id>$Name</id><name>$Name</name>
  <description>OpenProject native Windows $Mode</description>
  <executable>$escapedRoot\runtime\ruby\bin\ruby.exe</executable>
  <arguments>"$escapedRoot\service.rb" $Mode</arguments>
  <workingdirectory>$escapedRoot\openproject</workingdirectory>
  <depend>OpenProjectPostgreSQL</depend>
  <startmode>Automatic</startmode><delayedAutoStart>true</delayedAutoStart>
  <serviceaccount><domain>NT AUTHORITY</domain><user>LocalService</user></serviceaccount>
  <stoptimeout>45 sec</stoptimeout>
  <onfailure action="restart" delay="15 sec"/>
  <logpath>$logPath</logpath><log mode="roll-by-size"><sizeThreshold>10240</sizeThreshold><keepFiles>4</keepFiles></log>
</service>
"@
}

foreach ($required in @($ruby,(Join-Path $pgBin 'pg_ctl.exe'),(Join-Path $Root 'service.rb'),(Join-Path $app 'Gemfile.lock'),(Join-Path $Root 'services\winsw.exe'))) {
    if (!(Test-Path -LiteralPath $required -PathType Leaf)) { throw "Incomplete installation: $required" }
}
$frontend=Get-Content -LiteralPath (Join-Path $app 'config\frontend_assets.manifest.json') -Raw | ConvertFrom-Json
foreach($name in 'polyfills.js','main.js','styles.css','spot.css') {
    $asset=$frontend.PSObject.Properties[$name].Value
    if(!$asset -or !(Test-Path -LiteralPath (Join-Path $app "public\assets\frontend\$asset") -PathType Leaf)){throw "Compiled frontend asset missing: $name"}
}
$env:PATH = "$(Split-Path $ruby);$pgBin;$env:SystemRoot\System32;$env:SystemRoot"
$env:BUNDLE_PATH = Join-Path $Root 'runtime\gems'
$env:BUNDLE_GEMFILE = Join-Path $app 'Gemfile'
$env:BUNDLE_WITHOUT = 'development:test'
$env:RUBYOPT = '-EUTF-8'
Push-Location $app
try {
    Invoke-Checked $ruby @('script/windows/check.rb')
    Invoke-Checked (Join-Path $pgBin 'postgres.exe') @('--version')
    if ($CheckBundle) {
        Invoke-Checked $ruby @('script/windows/test_file_store.rb')
        Write-Output 'Native installation bundle verified without Git, Node, Docker or WSL on PATH.'
        exit 0
    }
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (!$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Run the installer as administrator.' }
    if ([version](Get-CimInstance Win32_OperatingSystem).Version -lt [version]'10.0.14393') { throw 'Windows Server 2016 / Windows 10 build 14393 or newer is required.' }

    $expected = @{
        OpenProjectPostgreSQL = Join-Path $pgBin 'pg_ctl.exe'
        OpenProjectWeb = Join-Path $Root 'services\openproject-web.exe'
        OpenProjectWorker = Join-Path $Root 'services\openproject-worker.exe'
    }
    foreach($name in $expected.Keys) {
        $existing = Get-CimInstance Win32_Service -Filter "Name='$name'"
        if ($existing -and !$existing.PathName.StartsWith(('"' + $expected[$name] + '"'),[StringComparison]::OrdinalIgnoreCase)) { throw "$name belongs to a different installation. It was not changed." }
    }
    $running = @($expected.Keys | Where-Object { (Get-Service $_ -ErrorAction SilentlyContinue).Status -eq 'Running' })
    if ($running.Count -eq 3) { Write-Output 'Existing running installation preserved.'; exit 0 }
    if ($running.Count) { throw 'Stop all OpenProject Manager services before repairing a partial installation.' }

    Protect-Directory $state
    Invoke-Checked icacls.exe @($state,'/grant','*S-1-5-20:RX','*S-1-5-19:RX')
    foreach($folder in 'logs','files','tmp') { Protect-Directory (Join-Path $state $folder) -ServiceSid 'S-1-5-19' }
    Protect-Directory (Join-Path $state 'postgresql-data') -ServiceSid 'S-1-5-20'
    Protect-Directory (Join-Path $state 'private')
    Invoke-Checked icacls.exe @((Join-Path $state 'private'),'/grant','*S-1-5-19:RX')
    foreach($folder in 'tmp','log') { New-Item -ItemType Directory -Force -Path (Join-Path $app $folder) | Out-Null; Invoke-Checked icacls.exe @((Join-Path $app $folder),'/grant','*S-1-5-19:(OI)(CI)F') }

    $managerFile = Join-Path $state 'manager.json'
    Assert-TrustedPath $managerFile
    if (!(Test-Path $managerFile)) { [IO.File]::WriteAllText($managerFile,(@{folder=$Root;host='127.0.0.1';port=8088} | ConvertTo-Json)) }
    Protect-File $managerFile -ServiceRead
    $manager = Get-Content -LiteralPath $managerFile -Raw | ConvertFrom-Json
    $bindAddress=$null
    if ($manager.folder -ne $Root -or ![Net.IPAddress]::TryParse($manager.host,[ref]$bindAddress) -or $bindAddress.AddressFamily -ne [Net.Sockets.AddressFamily]::InterNetwork -or $manager.port -lt 1 -or $manager.port -gt 65535) { throw 'Invalid manager settings; existing settings preserved.' }
    $webListener=New-Object Net.Sockets.TcpListener($bindAddress,[int]$manager.port)
    try { $webListener.Start() } finally { $webListener.Stop() }

    $data = Join-Path $state 'postgresql-data'
    $runtimeFile = Join-Path $state 'private\runtime.json'
    $adminFile = Join-Path $state 'private\database-admin.json'
    $loginFile = Join-Path $state 'login.txt'
    foreach($path in $runtimeFile,$adminFile,$loginFile){Assert-TrustedPath $path}
    if (!(Test-Path $runtimeFile)) {
        if (Test-Path (Join-Path $data 'PG_VERSION')) { throw 'Database exists without runtime credentials. Restore private/runtime.json from backup.' }
        @{DatabasePassword=New-Secret;SecretKeyBase=New-Secret} | ConvertTo-Json | Set-Content -LiteralPath $runtimeFile -Encoding ASCII
        @{Password=New-Secret} | ConvertTo-Json | Set-Content -LiteralPath $adminFile -Encoding ASCII
        @("URL: http://127.0.0.1:$($manager.port)",'Username: admin',"Initial password: $(New-Secret)") | Set-Content -LiteralPath $loginFile -Encoding ASCII
    }
    if (!(Test-Path $adminFile)) { throw 'Database administrator credential is missing. Restore it from backup.' }
    Protect-File $runtimeFile -ServiceRead
    Protect-File $adminFile
    if(Test-Path $loginFile){Protect-File $loginFile}
    $runtime = Get-Content -LiteralPath $runtimeFile -Raw | ConvertFrom-Json
    $databaseAdmin = Get-Content -LiteralPath $adminFile -Raw | ConvertFrom-Json
    foreach($secret in @($runtime.DatabasePassword,$runtime.SecretKeyBase,$databaseAdmin.Password)) { if($secret -notmatch '^[a-f0-9]{64}$'){throw 'Invalid stored credentials. Existing files preserved.'} }

    if (!(Test-Path (Join-Path $data 'PG_VERSION'))) {
        $listener = New-Object Net.Sockets.TcpListener([Net.IPAddress]::Loopback,15434)
        try { $listener.Start() } finally { $listener.Stop() }
        $passwordFile = Join-Path $state 'init-password'
        # initdb drops the Administrators SID, so its restricted token needs a temporary user grant.
        $initSid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value
        try {
            [IO.File]::WriteAllText($passwordFile,$databaseAdmin.Password)
            Protect-File $passwordFile
            Invoke-Checked icacls.exe @($passwordFile,'/grant',"*${initSid}:R")
            Invoke-Checked icacls.exe @($state,'/grant',"*${initSid}:RX")
            Invoke-Checked icacls.exe @($data,'/grant',"*${initSid}:(OI)(CI)F")
            Invoke-Checked (Join-Path $pgBin 'initdb.exe') @('-D',$data,'-U','openproject_admin','--encoding=UTF8','--locale=C','--locale-provider=icu','--icu-locale=en-US','--auth=scram-sha-256',"--pwfile=$passwordFile")
        } finally {
            Remove-Item -LiteralPath $passwordFile -Force -ErrorAction SilentlyContinue
            Protect-Directory $data -ServiceSid 'S-1-5-20'
            Invoke-Checked icacls.exe @($state,'/remove:g',"*$initSid")
        }
    }
    if ((Get-Content (Join-Path $data 'PG_VERSION') -Raw).Trim() -ne '17') { throw 'Expected PostgreSQL 17. Existing database was not changed.' }
    Protect-Directory $data -ServiceSid 'S-1-5-20'
    if (!(Get-Service OpenProjectPostgreSQL -ErrorAction SilentlyContinue)) {
        Add-Content -LiteralPath (Join-Path $data 'postgresql.auto.conf') -Encoding ASCII -Value "`nlisten_addresses = '127.0.0.1'`nport = 15434`nlogging_collector = on"
        Invoke-Checked (Join-Path $pgBin 'pg_ctl.exe') @('register','-N','OpenProjectPostgreSQL','-D',$data,'-U','NT AUTHORITY\NetworkService','-S','auto')
    }
    Start-Service OpenProjectPostgreSQL
    $env:PGPASSWORD = $databaseAdmin.Password
    try {
        $ready=$false
        for($attempt=0;$attempt -lt 45;$attempt++) { & (Join-Path $pgBin 'pg_isready.exe') -q -h 127.0.0.1 -p 15434; if($LASTEXITCODE -eq 0){$ready=$true;break}; Start-Sleep -Seconds 1 }
        if(!$ready){throw 'PostgreSQL startup timed out. Check the private PostgreSQL logs.'}
        $sql=@"
SELECT 'CREATE ROLE openproject LOGIN PASSWORD ''$($runtime.DatabasePassword)''' WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname='openproject')
\gexec
SELECT 'CREATE DATABASE openproject OWNER openproject ENCODING ''UTF8'' TEMPLATE template0' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname='openproject')
\gexec
"@
        $ErrorActionPreference='Continue'
        try { $sql | & (Join-Path $pgBin 'psql.exe') -X -w -h 127.0.0.1 -p 15434 -U openproject_admin -d postgres -v ON_ERROR_STOP=1 -q 2>$null | Out-Null }
        finally { $ErrorActionPreference='Stop' }
        if($LASTEXITCODE -ne 0){throw 'Database provisioning failed; SQL and passwords are not logged.'}
    } finally { Remove-Item Env:PGPASSWORD -ErrorAction SilentlyContinue }

    $setupLog = Join-Path $state 'logs\setup.log'
    Invoke-RubyTask 'migrate' $setupLog
    $initialized = Join-Path $state 'initialized'
    Assert-TrustedPath $initialized
    if(!(Test-Path $initialized)) {
        $initial = Get-Content -LiteralPath $loginFile | Where-Object {$_ -match '^Initial password: ([a-f0-9]{64})$'}
        if(!$initial){throw 'Initial login file missing or invalid; restore login.txt before resuming setup.'}
        $env:OPENPROJECT_SEED__ADMIN__USER__PASSWORD=$initial.Substring('Initial password: '.Length)
        $env:OPENPROJECT_SEED__ADMIN__USER__PASSWORD__RESET='false'
        $env:OPENPROJECT_SEED__ADMIN__USER__MAIL='admin@localhost.invalid'
        try { Invoke-RubyTask 'seed' $setupLog -Append }
        finally { Remove-Item Env:OPENPROJECT_SEED__ADMIN__USER__PASSWORD -ErrorAction SilentlyContinue }
        [IO.File]::WriteAllText($initialized,'17.8.0 Windows native')
    }
    foreach($mode in 'worker','web') {
        $serviceName = if($mode -eq 'web'){'OpenProjectWeb'}else{'OpenProjectWorker'}
        $wrapper=Join-Path $Root "services\openproject-$mode.exe"
        if(!(Test-Path $wrapper)){Copy-Item -LiteralPath (Join-Path $Root 'services\winsw.exe') -Destination $wrapper}
        if(!(Get-Service $serviceName -ErrorAction SilentlyContinue)) {
            [IO.File]::WriteAllText([IO.Path]::ChangeExtension($wrapper,'xml'),(Get-ServiceXml $serviceName $mode))
            Invoke-Checked $wrapper @('install')
        }
        Start-Service $serviceName
    }
    Write-Output 'OpenProject services installed. Wait for Web: Ready in the manager. Database and login details are under %PROGRAMDATA%\OpenProjectManager.'
} finally { Pop-Location }
