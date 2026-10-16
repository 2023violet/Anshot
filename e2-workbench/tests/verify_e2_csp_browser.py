#!/usr/bin/env python3
"""R08: actual Chrome CSP enforcement, local Rust/WASM, and blocked egress.
Only SHA256-locked synthetic fixture T01. Not a production deployment audit.
"""
import http.server
import json
import threading
from pathlib import Path
from playwright.sync_api import sync_playwright

# CSP-safe test polling: no unsafe-eval in the real application policy.
import time
def wait_csp(page, expression, arg=None, timeout=30000):
    until=time.monotonic()+timeout/1000
    while time.monotonic()<until:
        if page.evaluate(f"(value) => Boolean({expression})", arg):
            return
        page.wait_for_timeout(100)
    raise AssertionError("CSP-safe browser condition timeout: "+expression)

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "docs/research/fixtures/T01_CHAT.png"
OUT = Path("/tmp/e2-browser-evidence")
OUT.mkdir(parents=True, exist_ok=True)

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)
    def log_message(self, *_args):
        pass

server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
origin = f"http://127.0.0.1:{server.server_port}"
with sync_playwright() as pw:
    browser = pw.chromium.launch(channel="chrome", headless=True, args=["--no-sandbox"])
    try:
        context = browser.new_context(viewport={"width": 1280, "height": 900})
        page = context.new_page()
        requests = []
        errors = []
        page.on("request", lambda r: requests.append(r.url))
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.goto(origin + "/e2-workbench/index.html")
        wait_csp(page, "window.e2Debug?.ready===true")
        assert page.locator('meta[http-equiv="Content-Security-Policy"]').count() == 1
        page.evaluate("""() => {
          window.__cspViolations=[];
          document.addEventListener('securitypolicyviolation', e=>{
            window.__cspViolations.push({
              directive:e.violatedDirective,blocked:e.blockedURI
            });
          });
          const s=document.createElement('script');
          s.textContent='window.__untrustedInlineScriptDidRun=true';
          document.body.append(s);
        }""")
        wait_csp(page, "window.__cspViolations.some(x => x.directive.startsWith('script-src'))")
        assert page.evaluate("window.__untrustedInlineScriptDidRun === undefined")
        blocked = page.evaluate("""async () => {
          try {
            await fetch('https://example.invalid/blocked-egress-test', {method:'POST', body:'synthetic-only'});
            return false;
          } catch(e) { return true; }
        }""")
        assert blocked, "CSP allowed external fetch"
        wait_csp(page, "window.__cspViolations.some(x => x.directive==='connect-src')")
        assert all(url.startswith((origin + "/", "blob:" + origin + "/")) for url in requests), requests
        page.locator("#pick").set_input_files(str(FIXTURE))
        wait_csp(page, "window.e2Debug.stage==='edit'")
        page.locator("#createMask").click()
        page.locator("#check").click()
        wait_csp(page, "window.e2Debug.stage==='review' && window.e2Debug.outputBytes>0")
        assert not errors, errors
        evidence = {
            "policy":page.locator('meta[http-equiv="Content-Security-Policy"]').get_attribute("content"),
            "violations":page.evaluate("window.__cspViolations"),
            "untrusted_inline_script_blocked":True,
            "external_post_blocked":True,
            "rust_wasm_synthetic_review":True,
            "actual_outgoing_remote_requests":0,
            "limit":"Dev HTML meta CSP only; production response header, other browsers and deployments are unverified"
        }
        (OUT/"e2-csp-evidence.json").write_text(json.dumps(evidence,indent=2)+"\n")
        print("E2_R08_CSP_REAL_BROWSER_ENFORCED_AND_LOCAL_WASM_PASS",json.dumps(evidence))
        context.close()
    finally:
        browser.close()
        server.shutdown()
