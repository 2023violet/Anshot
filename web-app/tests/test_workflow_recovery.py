#!/usr/bin/env python3
"""V1.3-03 browser regressions for negative-state recovery.

All pictures are generated in memory. Uses real Chrome and real Rust/WASM.
Native event simulations are explicitly limited to JS bridge contract tests;
the separate Windows job runs the actual WebView2 host.
"""
import http.server
import io
import json
import threading
import time
from pathlib import Path
from PIL import Image
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[2]
OUT=Path("/tmp/shareready-workflow-v1303")
OUT.mkdir(parents=True,exist_ok=True)
def make(name,size,color):
    image=Image.new("RGB",size,color)
    stream=io.BytesIO()
    image.save(stream,"PNG")
    path=OUT/name
    path.write_bytes(stream.getvalue())
    return path

first=make("synthetic-first.png",(480,320),(226,239,247))
second=make("synthetic-second.png",(360,240),(174,203,227))
corrupt=OUT/"corrupt-image.png";corrupt.write_bytes(b"not a png, source stays")
oversized=OUT/"oversized-image.png";oversized.write_bytes(b"\x89PNG\r\n\x1a\n"+b"X"*(25*1024*1024))
class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self,*a,**kw):super().__init__(*a,directory=str(ROOT),**kw)
    def log_message(self,*a):pass
server=http.server.ThreadingHTTPServer(("127.0.0.1",0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
url=f"http://127.0.0.1:{server.server_port}"
cases=[]

def wait(page,expr,timeout=20000):
    end=time.monotonic()+timeout/1000
    while time.monotonic()<end:
        if page.evaluate(f"() => Boolean({expr})"): return
        page.wait_for_timeout(100)
    raise AssertionError("Timed out: "+expr)

def wait_dialog(page,choice,target,timeout=15000):
    end=time.monotonic()+timeout/1000
    while time.monotonic()<end:
        if choice["seen"]>=target:return
        page.wait_for_timeout(80)
    raise AssertionError("Expected confirmation dialog not delivered")

with sync_playwright() as pw:
    browser=pw.chromium.launch(channel="chrome",headless=True,args=["--no-sandbox"])
    try:
        context=browser.new_context(viewport={"width":1440,"height":900},accept_downloads=True)
        # Simulated messages test only the JS side of the Windows host protocol.
        context.add_init_script("""(() => {
          window.__shareReadyHostHandler=null;
          window.chrome=window.chrome||{};
          Object.defineProperty(window.chrome,"webview",{
             configurable:true,
             value:{ addEventListener(type,callback){
               if(type==="message")window.__shareReadyHostHandler=callback
             } }
          });
          window.__hostReply=(state,uri)=>window.__shareReadyHostHandler({
            data:{type:"native-download",state,uri}
          });
        })();""")
        page=context.new_page()
        errors=[]
        page.on("pageerror",lambda e:errors.append(str(e)))
        dialog_choice={"accept":False,"seen":0}
        def on_dialog(dialog):
            dialog_choice["seen"]+=1
            if dialog_choice["accept"]:dialog.accept()
            else:dialog.dismiss()
        page.on("dialog",on_dialog)
        page.goto(url+"/web-app/index.html")
        wait(page,"window.e2Debug?.ready===true")
        page.locator("#pick").set_input_files(str(first))
        wait(page,"window.e2Debug?.sourceSize[0]===480 && window.e2Debug.stage==='edit'")
        page.locator("#createMask").click()
        old_mask=page.evaluate("window.e2Debug.regions")
        old_revision=page.evaluate("window.e2Debug.revision")
        assert len(old_mask)==1

        # Invalid and excessive inputs must not mutate the current edit.
        for bad in (corrupt,oversized):
            page.locator("#pick").set_input_files(str(bad))
            wait(page,"document.getElementById('status').textContent.includes('导入失败')")
            assert page.evaluate("window.e2Debug.regions")==old_mask
            assert page.evaluate("window.e2Debug.revision")==old_revision
            assert page.evaluate("window.e2Debug.sourceSize")==[480,320]
        cases.append("invalid and oversized PNG preserve full edit/revision")

        # Decode valid replacement and cancel *after* validating candidate:
        dialog_choice["accept"]=False
        prior=dialog_choice["seen"]
        page.locator("#pick").set_input_files(str(second))
        wait_dialog(page,dialog_choice,prior+1)
        assert page.evaluate("window.e2Debug.sourceSize")==[480,320]
        assert page.evaluate("window.e2Debug.regions")==old_mask
        cases.append("cancel replacement preserves source and masks")

        page.locator("#check").click()
        wait(page,"window.e2Debug.stage==='review' && window.e2Debug.outputBytes>0")
        active_url=page.evaluate("window.e2Debug.outputUrl")
        assert active_url.startswith("blob:")
        # Each click must use a fresh, single-use blob URL (WebView2 does
        # not reliably re-download an already-canceled blob address).
        with page.expect_download():
            page.locator("#download").click()
        first_attempt=page.evaluate("window.e2Debug.pendingDownloadUrl")
        assert first_attempt.startswith("blob:") and first_attempt!=active_url
        assert page.evaluate("window.e2Debug.nativeDownloadState")=="pending"
        assert page.locator("#download").is_disabled()
        page.evaluate("uri => window.__hostReply('saved',uri)",first_attempt+"-old")
        assert page.evaluate("window.e2Debug.nativeDownloadState")=="pending"
        page.evaluate("uri => window.__hostReply('failed',uri)",first_attempt)
        assert page.evaluate("window.e2Debug.nativeDownloadState")=="failed"
        assert page.evaluate("window.e2Debug.pendingDownloadUrl")==None
        assert page.locator("#download").is_enabled()
        assert "保存失败" in page.locator("#reviewStatus").inner_text()
        with page.expect_download():
            page.locator("#download").click()
        canceled_attempt=page.evaluate("window.e2Debug.pendingDownloadUrl")
        assert canceled_attempt!=first_attempt
        assert page.locator("#download").is_disabled()
        page.evaluate("uri => window.__hostReply('canceled',uri)",canceled_attempt)
        assert page.evaluate("window.e2Debug.nativeDownloadState")=="canceled"
        assert page.evaluate("window.e2Debug.pendingDownloadUrl")==None
        assert page.locator("#download").is_enabled()
        assert "取消" in page.locator("#reviewStatus").inner_text()
        with page.expect_download():
            page.locator("#download").click()
        saved_attempt=page.evaluate("window.e2Debug.pendingDownloadUrl")
        assert saved_attempt not in (first_attempt,canceled_attempt,active_url)
        page.evaluate("uri => window.__hostReply('saved',uri)",saved_attempt)
        assert page.evaluate("window.e2Debug.nativeDownloadState")=="saved"
        assert "已保存" in page.locator("#reviewStatus").inner_text()
        cases.append("unique blob on retry; wrong URL ignored; failure/cancel recover")

        # Clipboard failure never triggers PNG downgrade to original file.
        page.evaluate("""() => {
          Object.defineProperty(navigator.clipboard,'write',{
            configurable:true,value:async()=>{throw new DOMException('Denied','NotAllowedError')}
          });
        }""")
        page.locator("#copy").click()
        wait(page,"window.e2Debug.copyState==='denied_or_unavailable'")
        assert page.locator("#copy").is_enabled()
        assert "复制失败" in page.locator("#reviewStatus").inner_text()
        assert page.evaluate("window.e2Debug.outputUrl")==active_url
        cases.append("clipboard denied: PNG stays valid and retry remains enabled")

        # Delayed previous PNG clipboard promise must not poison a newer
        # output's success or UI after the old review has been invalidated.
        page.evaluate("""() => {
          window.__lateClipboardStarted=false;
          Object.defineProperty(navigator.clipboard,'write',{
            configurable:true,value:()=>new Promise(resolve=>{
              window.__lateClipboardStarted=true;window.__resolveLateClipboard=resolve
            })
          });
        }""")
        page.locator("#copy").click()
        wait(page,"window.__lateClipboardStarted===true")
        page.locator("#back").click()
        page.locator("#regionList .region").click()
        page.locator("#precX").fill("15")
        page.locator("#precX").press("Tab")
        assert page.evaluate("window.e2Debug.outputRevision")==-1
        assert page.evaluate("window.e2Debug.revision")>old_revision
        page.evaluate("uri => window.__hostReply('saved',uri)",saved_attempt)
        assert page.evaluate("window.e2Debug.nativeDownloadState")=="not_attempted"
        assert page.locator("#download").is_disabled()
        page.evaluate("window.__resolveLateClipboard()")
        page.wait_for_timeout(120)
        assert page.evaluate("window.e2Debug.copyState")=="not_attempted"
        assert "PNG 写入剪贴板成功" not in page.locator("#status").inner_text()
        cases.append("old download/clipboard callbacks cannot affect edited image")

        # A fresh review generates a new URL, old canceled save must not unlock
        # the fresh in-flight download. Replacement clears old masks/history.
        page.locator("#check").click()
        wait(page,"window.e2Debug.stage==='review'")
        current_url=page.evaluate("window.e2Debug.outputUrl")
        assert current_url!=active_url
        with page.expect_download():page.locator("#download").click()
        assert page.locator("#download").is_disabled()
        current_attempt=page.evaluate("window.e2Debug.pendingDownloadUrl")
        assert current_attempt not in (first_attempt,canceled_attempt,saved_attempt)
        page.evaluate("uri => window.__hostReply('canceled',uri)",saved_attempt)
        assert page.locator("#download").is_disabled()
        page.evaluate("uri => window.__hostReply('canceled',uri)",current_attempt)
        assert page.locator("#download").is_enabled()

        # Replacement while review is open (simulated file event) preserves
        # old image on cancel and invalidates all previous revisions on accept.
        dialog_choice["accept"]=False
        prior=dialog_choice["seen"]
        page.locator("#pick").set_input_files(str(second))
        wait_dialog(page,dialog_choice,prior+1)
        assert page.evaluate("window.e2Debug.stage")=="review"
        assert page.evaluate("window.e2Debug.outputUrl")==current_url
        dialog_choice["accept"]=True
        prior=dialog_choice["seen"]
        page.locator("#pick").set_input_files(str(second))
        wait_dialog(page,dialog_choice,prior+1)
        wait(page,"window.e2Debug.stage==='edit' && window.e2Debug.sourceSize[0]===360")
        assert page.evaluate("window.e2Debug.regions")==[]
        assert page.evaluate("window.e2Debug.outputRevision")==-1
        assert page.locator("#check").is_disabled()
        assert page.locator("#download").is_disabled()
        assert page.evaluate("window.e2Debug.nativeDownloadState")=="not_attempted"
        page.evaluate("uri => window.__hostReply('saved',uri)",current_attempt)
        assert page.evaluate("window.e2Debug.nativeDownloadState")=="not_attempted"
        cases.append("review replacement cancel/accept and late native completion are fail-closed")

        assert not errors,errors
        (OUT/"workflow-results.json").write_text(json.dumps({
          "result":"PASS","real_browser":"Chrome","synthetic_only":True,
          "cases":cases,"page_errors":errors},ensure_ascii=False,indent=2)+"\n")
        context.close()
    finally:
        browser.close()
        server.shutdown()
print("SHAREREADY_V1303_NEGATIVE_RECOVERY_BROWSER_PASS",json.dumps(cases,ensure_ascii=False))
