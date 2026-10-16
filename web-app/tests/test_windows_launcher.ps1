# Windows-only native integration test for the PowerShell localhost launcher.
# CI supplies harmless placeholder pkg assets here; Rust WASM is built and
# functionally tested in the separate Linux/Chrome product job.
$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$script=Join-Path $root 'Start-ShareReady.ps1'
$port=28000+(Get-Random -Minimum 0 -Maximum 18000)
$out=Join-Path $env:TEMP 'share-ready-windows-stdout.txt'
$err=Join-Path $env:TEMP 'share-ready-windows-stderr.txt'
$process=Start-Process -FilePath 'powershell.exe' -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$script`" -NoBrowser -Port $port" -PassThru -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError $err

function Fetch-Local([string]$resource, [string]$method='GET') {
  $req=[System.Net.HttpWebRequest]::Create("http://127.0.0.1:$port$resource")
  $req.Method=$method
  $req.Timeout=3500
  $req.AllowAutoRedirect=$false
  try {
    $resp=[System.Net.HttpWebResponse]$req.GetResponse()
  } catch [System.Net.WebException] {
    if ($null -eq $_.Exception.Response) { throw }
    $resp=[System.Net.HttpWebResponse]$_.Exception.Response
  }
  try {
    $body=[System.IO.MemoryStream]::new()
    try {
      if ($method -ne 'HEAD') {
        $stream=$resp.GetResponseStream()
        $stream.CopyTo($body)
      }
      return @{
        Status=[int]$resp.StatusCode
        Type=[string]$resp.ContentType
        Bytes=$body.ToArray()
        CSP=[string]$resp.Headers['Content-Security-Policy']
        XContentType=[string]$resp.Headers['X-Content-Type-Options']
        Cache=[string]$resp.Headers['Cache-Control']
      }
    } finally { $body.Dispose() }
  } finally { $resp.Dispose() }
}

try {
  $started=$false
  for ($i=0;$i -lt 60;$i++){
    Start-Sleep -Milliseconds 150
    if ($process.HasExited) {
      throw "Local launcher exited early: $(Get-Content $err -Raw -ErrorAction SilentlyContinue)"
    }
    try {
      $r=Fetch-Local '/'
      if ($r.Status -eq 200) { $started=$true; break }
    } catch { }
  }
  if (-not $started) {throw "Localhost server did not respond"}

  $rootResponse=Fetch-Local '/'
  if ($rootResponse.Status -ne 200) {throw "Index status not 200"}
  $page=[Text.Encoding]::UTF8.GetString($rootResponse.Bytes)
  if (-not $page.Contains('ShareReady')) {throw "Not the ShareReady index"}
  if (-not $rootResponse.CSP.Contains("default-src 'none'")) {throw "HTTP security CSP header missing"}
  if ($rootResponse.XContentType -ne 'nosniff') {throw "nosniff missing"}
  if ($rootResponse.Cache -ne 'no-store') {throw "no-store missing"}

  $javascript=Fetch-Local '/app.js'
  if ($javascript.Status -ne 200 -or -not $javascript.Type.Contains('javascript')) {throw "JS served incorrectly"}
  $wasm=Fetch-Local '/pkg/shareready_e1_bg.wasm'
  if ($wasm.Status -ne 200 -or $wasm.Type -ne 'application/wasm') {throw "WASM MIME missing"}
  if ((Fetch-Local '/secret.txt').Status -ne 404) {throw "Arbitrary file path is exposed"}
  if ((Fetch-Local '/e1-spike/src/lib.rs').Status -ne 404) {throw "Source-file path is exposed"}
  if ((Fetch-Local '/app.js' 'POST').Status -ne 405) {throw "HTTP POST unexpectedly allowed"}
  if ((Fetch-Local '/index.html' 'HEAD').Status -ne 200) {throw "HEAD unsupported"}

  $sockets=Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction Stop
  if (@($sockets | Where-Object { $_.LocalAddress -ne '127.0.0.1' }).Count -gt 0) {
    throw "Unexpected non-loopback listener"
  }
  Write-Host "WINDOWS_NATIVE_LOOPBACK_HTTP_CSP_WASM_POST_DENY_PASS PORT=$port"
} finally {
  if ($null -ne $process -and -not $process.HasExited) {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
  }
}
