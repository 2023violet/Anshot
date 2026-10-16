# ShareReady: localhost-only static-file server for Windows 10/11.
# Never reads arbitrary file paths, accepts uploads, or connects to the Internet.
param(
  [ValidateRange(0,65535)][int]$Port = 0,
  [switch]$NoBrowser
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $PSScriptRoot).Path
$required = @('index.html','app.js','pkg/shareready_e1.js','pkg/shareready_e1_bg.wasm')
foreach ($file in $required) {
  if (-not (Test-Path -LiteralPath (Join-Path $root ($file -replace '/', [IO.Path]::DirectorySeparatorChar)) -PathType Leaf)) {
    [Console]::Error.WriteLine("ShareReady missing application asset: $file")
    exit 3
  }
}
$mime = @{
  '/index.html' = 'text/html; charset=utf-8'
  '/app.js' = 'text/javascript; charset=utf-8'
  '/pkg/shareready_e1.js' = 'text/javascript; charset=utf-8'
  '/pkg/shareready_e1_bg.wasm' = 'application/wasm'
}
$routes = @{
  '/' = 'index.html'
  '/index.html' = 'index.html'
  '/app.js' = 'app.js'
  '/pkg/shareready_e1.js' = 'pkg/shareready_e1.js'
  '/pkg/shareready_e1_bg.wasm' = 'pkg/shareready_e1_bg.wasm'
}
$csp = "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self' blob:; img-src 'self' blob:; style-src 'unsafe-inline'; font-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'; worker-src 'none'"
$listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
$listener.Start()
$boundPort = ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
$url = "http://127.0.0.1:$boundPort/"
Write-Host "ShareReady is running locally: $url"
Write-Host "Close this window or press Ctrl+C to stop. No external network listener."
if (-not $NoBrowser) {
  try { Start-Process $url } catch {
    Write-Warning "Could not open a browser automatically. Visit $url in Chrome or Edge."
  }
}
try {
  while ($true) {
    $client = $listener.AcceptTcpClient()
    try {
      $client.ReceiveTimeout = 4000
      $client.SendTimeout = 4000
      $stream = $client.GetStream()
      $reader = [System.IO.StreamReader]::new($stream, [System.Text.Encoding]::ASCII, $false, 1024, $true)
      $line = $reader.ReadLine()
      $status = 400
      $reason = 'Bad Request'
      $method = ''
      $path = ''
      if ($null -ne $line -and $line.Length -le 2048 -and $line -match '^(GET|HEAD|POST|PUT|DELETE|OPTIONS) ([^ ]+) HTTP/1\.[01]$') {
        $method = $Matches[1]; $path = $Matches[2]
        $headerCount = 0
        do {
          $headerLine = $reader.ReadLine()
          $headerCount++
          if ($headerCount -gt 64) { break }
        } while ($null -ne $headerLine -and $headerLine.Length -gt 0)
        if ($headerCount -le 64) {
          if ($method -ne 'GET' -and $method -ne 'HEAD') {
            $status = 405; $reason = 'Method Not Allowed'
          } elseif ($routes.ContainsKey($path)) {
            $status = 200; $reason = 'OK'
          } else {
            $status = 404; $reason = 'Not Found'
          }
        }
      }
      $payload = [byte[]]@()
      $contentType = 'text/plain; charset=utf-8'
      if ($status -eq 200) {
        $relative = [string]$routes[$path]
        $filename = Join-Path $root ($relative -replace '/', [IO.Path]::DirectorySeparatorChar)
        $payload = [System.IO.File]::ReadAllBytes($filename)
        $typePath = if ($path -eq '/') { '/index.html' } else { $path }
        $contentType = $mime[$typePath]
      } else {
        $payload = [System.Text.Encoding]::UTF8.GetBytes("$status $reason")
      }
      $headers = @(
        "HTTP/1.1 $status $reason",
        "Content-Type: $contentType",
        "Content-Length: $($payload.Length)",
        "Connection: close",
        "Cache-Control: no-store",
        "X-Content-Type-Options: nosniff",
        "Referrer-Policy: no-referrer",
        "Cross-Origin-Resource-Policy: same-origin",
        "Content-Security-Policy: $csp",
        "Allow: GET, HEAD",
        "",
        ""
      ) -join "`r`n"
      $headBytes = [System.Text.Encoding]::ASCII.GetBytes($headers)
      $stream.Write($headBytes,0,$headBytes.Length)
      if ($method -ne 'HEAD' -and $payload.Length -gt 0) {
        $stream.Write($payload,0,$payload.Length)
      }
      $stream.Flush()
      $reader.Dispose()
    } catch {
      Write-Warning "Request rejected: $($_.Exception.Message)"
    } finally {
      $client.Close()
    }
  }
} finally {
  $listener.Stop()
}
