#!/usr/bin/env python3
"""Test real clipboard support vs denied state, genuine fallback, performance/memory on synthetic fixture."""
import http.server,json,threading
from pathlib import Path
from playwright.sync_api import sync_playwright
ROOT=Path(__file__).resolve().parents[2]
OUT=Path("/tmp/e1-web-browser");OUT.mkdir(exist_ok=True)
class H(http.server.SimpleHTTPRequestHandler):
 def __init__(self,*a,**kw):super().__init__(*a,directory=str(ROOT),**kw)
 def log_message(self,*a):pass
server=http.server.ThreadingHTTPServer(("127.0.0.1",0),H)
threading.Thread(target=server.serve_forever,daemon=True).start()
trials=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(headless=True,args=["--no-sandbox","--enable-precise-memory-info"])
 try:
  for mode in ["default_browser_permissions","grant_clipboard_read_write"]:
   ctx=browser.new_context(accept_downloads=True)
   if mode.startswith("grant"):
    try:ctx.grant_permissions(["clipboard-read","clipboard-write"],origin=f"http://127.0.0.1:{server.server_port}")
    except Exception as e:print("GRANT_UNAVAILABLE",str(e))
   p=ctx.new_page()
   p.goto(f"http://127.0.0.1:{server.server_port}/e1-spike/web/index.html")
   p.wait_for_function("window.e1Debug?.ready === true")
   p.locator("#profile").select_option("T04_EDGES")
   p.locator("#file").set_input_files(str(ROOT/"docs/research/fixtures/T04_EDGES.png"))
   p.locator("#generate").click()
   p.wait_for_function("!document.getElementById('copy').disabled",timeout=45000)
   p.locator("#copy").click()
   p.wait_for_function("window.e1Debug?.copiedStatus !== 'not_attempted'",timeout=15000)
   state=p.evaluate("window.e1Debug")
   assert state["copiedStatus"] in ("write_and_readback_exact","write_succeeded_readback_unverified","denied_or_unavailable")
   if state["copiedStatus"]=="denied_or_unavailable":
    assert "Download PNG instead" in p.locator("#status").inner_text()
   with p.expect_download() as ev:p.locator("#download").click()
   ev.value.save_as(OUT/(mode+".png"))
   mem=p.evaluate("({js_heap_bytes:performance.memory?.usedJSHeapSize??null,js_heap_limit_bytes:performance.memory?.jsHeapSizeLimit??null})")
   trials.append({"mode":mode,"clipboard_result":state["copiedStatus"],"clipboard_readback_error":state.get("readbackError"),"memory":mem,
                  "WASM_render_ms":state["renderMs"],"output_bytes":state["outputBytes"],
                  "downloaded_png_bytes":(OUT/(mode+".png")).stat().st_size})
   print("E1_ACTUAL_CLIPBOARD_CONDITION",json.dumps(trials[-1]))
   ctx.close()
 finally:
  browser.close()
  server.shutdown()
(OUT/"clipboard_memory_evidence.json").write_text(json.dumps({"trials":trials,"limitations":"headless Chrome, measured and denial cases as observed; not a cross-browser certification"},indent=2)+"\n")
assert len(trials)==2
print("E1_CLIPBOARD_ALLOW_DENY_AND_DOWNLOAD_FALLBACK_ACTUAL_CONDITION_RECORDED")
