# V12-01: RED means the original unchanged Windows launcher serves a harmless fixture
# on forbidden HTTP requests. Exit zero ONLY if the insecure baseline is reproducible.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$source = (Resolve-Path (Join-Path $PSScriptRoot '../Start-ShareReady.ps1')).Path
$hash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
$tmp = Join-Path ([IO.Path]::GetTempPath()) ('shareready-red-' + [guid]::NewGuid().ToString('N'))
$root = Join-Path $tmp 'web-app'
$pkg = Join-Path $root 'pkg'
[IO.Directory]::CreateDirectory($pkg) | Out-Null
Copy-Item -LiteralPath $source -Destination (Join-Path $root 'Start-ShareReady.ps1')
$marker = 'V12-RED-SYNTHETIC-9C41E'
[IO.File]::WriteAllText((Join-Path $root 'index.html'),("<html>ShareReady $marker</html>"),[Text.Encoding]::UTF8)
[IO.File]::WriteAllText((Join-Path $root 'app.js'),'synthetic-js',[Text.Encoding]::UTF8)
[IO.File]::WriteAllText((Join-Path $pkg 'shareready_e1.js'),'synthetic-glue',[Text.Encoding]::UTF8)
[IO.File]::WriteAllBytes((Join-Path $pkg 'shareready_e1_bg.wasm'),[byte[]](0,97,115,109))
$free = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback,0)
$free.Start()
$port = ([Net.IPEndPoint]$free.LocalEndpoint).Port
$free.Stop()
$nl = [string][char]13 + [string][char]10
$end = $nl + $nl
$validHost = "Host: 127.0.0.1:$port"
$logOut = Join-Path $tmp 'stdout.log'
$logErr = Join-Path $tmp 'stderr.log'
$process = $null
$observed = [System.Collections.Generic.List[object]]::new()

function Send-Raw {
  param([string]$Name,[string]$Wire)
  $client=[Net.Sockets.TcpClient]::new()
  try {
    $client.ReceiveTimeout=6500
    $client.SendTimeout=3000
    $client.Connect([Net.IPAddress]::Loopback,[int]$port)
    $stream=$client.GetStream()
    $stream.ReadTimeout=6500
    $data=[Text.Encoding]::ASCII.GetBytes($Wire)
    $stream.Write($data,0,$data.Length)
    $client.Client.Shutdown([Net.Sockets.SocketShutdown]::Send)
    $buf=New-Object byte[] 8192
    $memory=[IO.MemoryStream]::new()
    try {
      while ($true) {
        $n=$stream.Read($buf,0,$buf.Length)
        if ($n -eq 0) { break }
        $memory.Write($buf,0,$n)
        if ($memory.Length -gt 262144) { throw "V12_UNEXPECTED_RESPONSE_SIZE:$Name" }
      }
      $response=[Text.Encoding]::UTF8.GetString($memory.ToArray())
      $responseLength=$memory.Length
    } finally { $memory.Dispose() }
    $status=0
    if ($response -match '^HTTP/1\.[01] ([0-9]{3})') { $status=[int]$Matches[1] }
    $leaked=$response.Contains($marker)
    $o=[pscustomobject]@{
      case=$Name; status=$status; synthetic_index_served=$leaked
      request_bytes=$data.Length; response_bytes=$responseLength
      request_sha256=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($data))
    }
    $observed.Add($o)
    Write-Host ("V12_RAW_OBSERVED {0} status={1} synthetic_served={2}" -f $Name,$status,$leaked)
    return $o
  } finally { $client.Close();$client.Dispose() }
}
function Expect-Red {
  param([object]$Result)
  if ($Result.status -ne 200 -or -not $Result.synthetic_index_served) {
    throw ("V12_RED_NOT_REPRODUCED:{0} status={1} served={2}" -f $Result.case,$Result.status,$Result.synthetic_index_served)
  }
}
try {
  $scriptCopy=Join-Path $root 'Start-ShareReady.ps1'
  $arguments=('-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + $scriptCopy + '" -NoBrowser -Port ' + $port)
  $process=Start-Process -FilePath 'powershell.exe' -ArgumentList $arguments -PassThru -WindowStyle Hidden -RedirectStandardOutput $logOut -RedirectStandardError $logErr
  $ready=$false
  for ($i=0;$i -lt 60;$i++) {
    Start-Sleep -Milliseconds 150
    if ($process.HasExited) { throw ("V12_LAUNCHER_EXITED:" + (Get-Content -LiteralPath $logErr -Raw -ErrorAction SilentlyContinue)) }
    try {
      $probe=Send-Raw 'N01_VALID_GET' ('GET / HTTP/1.1' + $nl + $validHost + $end)
      if ($probe.status -eq 200 -and $probe.synthetic_index_served) { $ready=$true;break }
    } catch { if ($i -eq 59) { throw } }
  }
  if (-not $ready) { throw 'V12_LAUNCHER_NOT_READY' }

  Expect-Red (Send-Raw 'N02_FOREIGN_HOST' ('GET / HTTP/1.1'+$nl+'Host: hostile.example.invalid'+$end))
  Expect-Red (Send-Raw 'N03_MISSING_HOST' ('GET / HTTP/1.1'+$end))
  Expect-Red (Send-Raw 'N04_DUPLICATE_HOST' ('GET / HTTP/1.1'+$nl+$validHost+$nl+'Host: hostile.example.invalid'+$end))
  Expect-Red (Send-Raw 'N05_HTTP10_MISSING_HOST' ('GET / HTTP/1.0'+$end))
  Expect-Red (Send-Raw 'N06_WRONG_PORT_HOST' ('GET / HTTP/1.1'+$nl+'Host: 127.0.0.1:1'+$end))
  Expect-Red (Send-Raw 'N07_OVERSIZE_HEADER_VALUE' ('GET / HTTP/1.1'+$nl+$validHost+$nl+'X-Test: '+('A'*16384)+$end))
  Expect-Red (Send-Raw 'N08_EOF_WITHOUT_FINAL_BLANK_LINE' ('GET / HTTP/1.1'+$nl+$validHost+$nl))

  $post=Send-Raw 'N09_POST_405' ('POST / HTTP/1.1'+$nl+$validHost+$nl+'Content-Length: 0'+$end)
  if ($post.status -ne 405 -or $post.synthetic_index_served) { throw 'V12_POST_CONTROL_FAILED' }
  $missing=Send-Raw 'N10_NON_ALLOWLIST_404' ('GET /private.txt HTTP/1.1'+$nl+$validHost+$end)
  if ($missing.status -ne 404 -or $missing.synthetic_index_served) { throw 'V12_ALLOWLIST_CONTROL_FAILED' }
  $head=Send-Raw 'N11_VALID_HEAD' ('HEAD / HTTP/1.1'+$nl+$validHost+$end)
  if ($head.status -ne 200 -or $head.synthetic_index_served) { throw 'V12_HEAD_CONTROL_FAILED' }

  Write-Host 'V12_01_RED_REPRODUCED_SEVEN_FORBIDDEN_EXPOSURES_THREE_CONTROLS_PASS'
} finally {
  if ($null -ne $process -and -not $process.HasExited) {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
  }
  $evidencePath=if ($env:RUNNER_TEMP) {Join-Path $env:RUNNER_TEMP 'v12-01-red-evidence.json'} else {Join-Path $tmp 'v12-01-red-evidence.json'}
  $result=[ordered]@{
    schema='v12-01-red-v1'
    synthetic_only=$true
    source_launcher_sha256=$hash
    assessment='RED baseline reproduced only; no security fix; no user data'
    probes=@($observed.ToArray())
  }
  [IO.File]::WriteAllText($evidencePath,($result | ConvertTo-Json -Depth 8),[Text.Encoding]::UTF8)
  Write-Host "V12_01_EVIDENCE=$evidencePath"
  Write-Host "V12_01_SOURCE_SHA256=$hash"
}
