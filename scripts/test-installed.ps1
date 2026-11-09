param([Parameter(Mandatory=$true)][string]$ResultFile)
$ErrorActionPreference='Stop'
try {
    $state=Join-Path $env:PROGRAMDATA 'OpenProjectManager'
    $settings=Get-Content (Join-Path $state 'manager.json') -Raw | ConvertFrom-Json
    $root=$settings.folder
    $services=@('OpenProjectPostgreSQL','OpenProjectWorker','OpenProjectWeb')
    foreach($name in $services){
        $service=Get-Service $name
        if($service.Status -ne 'Running'){throw "$name is not running"}
    }
    $url="http://127.0.0.1:$($settings.port)"
    $ready=$false
    for($attempt=0;$attempt -lt 120;$attempt++){
        try{$response=Invoke-WebRequest "$url/login" -UseBasicParsing -SessionVariable session -TimeoutSec 5; if($response.StatusCode -eq 200){$ready=$true;break}}catch{}
        Start-Sleep -Seconds 2
    }
    if(!$ready){throw 'Installed web service did not become ready'}
    $initial=Get-Content (Join-Path $state 'login.txt') | Where-Object {$_ -match '^Initial password: ([a-f0-9]{64})$'}
    if(!$initial){throw 'Missing initial login credential'}
    $password=$initial.Substring('Initial password: '.Length)
    $token=[regex]::Match($response.Content,'name="authenticity_token"[^>]*value="([^"]+)"').Groups[1].Value
    if(!$token){throw 'Missing CSRF login token'}
    $logged=Invoke-WebRequest "$url/login" -UseBasicParsing -WebSession $session -Method Post -Body @{username='admin';password=$password;authenticity_token=[Net.WebUtility]::HtmlDecode($token)} -TimeoutSec 60
    if($logged.StatusCode -ne 200 -or $logged.Content -notmatch 'OpenProject Admin'){throw 'Installed service login failed'}
    $identity=Invoke-WebRequest "$url/api/v3/users/me" -UseBasicParsing -WebSession $session -TimeoutSec 30
    $identityJson=if($identity.Content -is [byte[]]){[Text.Encoding]::UTF8.GetString($identity.Content)}else{$identity.Content}
    $currentUser=$identityJson | ConvertFrom-Json
    if($currentUser.login -ne 'admin' -or $currentUser._type -ne 'User'){throw "Session identity check failed: type=$($currentUser._type), login=$($currentUser.login), name=$($currentUser.name), id=$($currentUser.id)"}
    $asset=[regex]::Match($logged.Content,'src="([^"]*assets/frontend/main-[^"]+\.js)"').Groups[1].Value
    if(!$asset -or (Invoke-WebRequest ([uri]::new([uri]$url,$asset)) -UseBasicParsing -TimeoutSec 30).StatusCode -ne 200){throw 'Installed frontend asset failed'}
    $runtime=Get-Content (Join-Path $state 'private\runtime.json') -Raw | ConvertFrom-Json
    $env:PGPASSWORD=$runtime.DatabasePassword
    $completed=& (Join-Path $root 'runtime\postgresql\bin\psql.exe') -X -w -h 127.0.0.1 -p 15434 -U openproject -d openproject -Atc 'SELECT count(*) FROM good_jobs WHERE performed_at IS NOT NULL;'
    if($LASTEXITCODE -ne 0 -or [int]$completed -eq 0){throw 'No completed background jobs'}
    $accounts=Get-CimInstance Win32_Service | Where-Object {$_.Name -in $services} | Select-Object Name,State,StartName,ProcessId
    @{Passed=$true;Url=$url;CompletedJobs=[int]$completed;Services=@($accounts);Checks=@('SCM running','CSRF admin login','frontend JavaScript','background jobs')} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $ResultFile -Encoding UTF8
} catch {
    @{Passed=$false;Error=$_.Exception.Message} | ConvertTo-Json | Set-Content -LiteralPath $ResultFile -Encoding UTF8
    exit 1
} finally {Remove-Item Env:PGPASSWORD -ErrorAction SilentlyContinue}
