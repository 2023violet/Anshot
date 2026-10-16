#!/usr/bin/env python3
"""V1 actual Chrome integration: arbitrary local PNG/JPEG, no fixture whitelist,
Rust EXIF-normalized preview, manual redaction, undo, PNG output and CSP.
Everything generated here is synthetic. No personal image data is used.
"""
import hashlib
import http.server
import io
import json
import threading
import time
from pathlib import Path
from PIL import Image, ImageDraw
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[2]
OUT=Path("/tmp/shareready-v1-browser")
OUT.mkdir(parents=True,exist_ok=True)

def make_png():
    image=Image.new("RGB",(640,400),(235,242,250))
    p=ImageDraw.Draw(image)
    p.rectangle((35,30,195,88),fill=(200,45,60))
    p.rectangle((350,290,560,356),fill=(30,88,175))
    buffer=io.BytesIO()
    image.save(buffer,format="PNG")
    path=OUT/"not-in-the-original-four-fixtures.png"
    path.write_bytes(buffer.getvalue())
    return path

def make_exif_jpeg():
    im=Image.new("RGB",(80,50),(210,225,239))
    ImageDraw.Draw(im).rectangle((2,2,20,15),fill=(240,60,40))
    exif=Image.Exif()
    exif[274]=6
    exif[270]="PRIVATE-SYNTHETIC-DESCRIPTION"
    path=OUT/"another-user-photo.jpg"
    im.save(path,"JPEG",quality=95,exif=exif)
    return path

def wait(page,js,timeout=30000):
    until=time.monotonic()+timeout/1000
    while time.monotonic()<until:
        if page.evaluate(f"() => Boolean({js})"):
            return
        page.wait_for_timeout(90)
    raise AssertionError(f"timeout: {js}")

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self,*args,**kwargs):
        super().__init__(*args,directory=str(ROOT),**kwargs)
    def log_message(self,*args):
        pass

server=http.server.ThreadingHTTPServer(("127.0.0.1",0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
origin=f"http://127.0.0.1:{server.server_port}"
results=[]
with sync_playwright() as pw:
    browser=pw.chromium.launch(channel="chrome",headless=True,args=["--no-sandbox"])
    try:
        context=browser.new_context(viewport={"width":1440,"height":900},accept_downloads=True)
        page=context.new_page()
        outgoing=[];errors=[]
        page.on("request",lambda request:outgoing.append(request.url))
        page.on("pageerror",lambda error:errors.append(str(error)))
        page.goto(origin+"/web-app/index.html")
        wait(page,"window.e2Debug?.ready===true")
        png=make_png()
        page.locator("#pick").set_input_files(str(png))
        wait(page,"window.e2Debug?.stage==='edit' && window.e2Debug.sourceSize[0]===640")
        assert "not-in-the-original-four-fixtures.png" in page.locator("#imageName").inner_text()
        assert page.locator("#check").is_disabled()
        page.locator("#createMask").click()
        assert len(page.evaluate("window.e2Debug.regions"))==1
        page.keyboard.press("Control+z")
        assert len(page.evaluate("window.e2Debug.regions"))==0
        page.keyboard.press("Control+Shift+z")
        assert len(page.evaluate("window.e2Debug.regions"))==1
        # Real 100% original pixel zoom, undoable bulk clear, and confirmation.
        page.locator("#zoomActual").click()
        assert page.evaluate("window.e2Debug.zoom")==1
        page.locator("#zoomFit").click()
        assert 0<page.evaluate("window.e2Debug.zoom")<=1
        page.once("dialog",lambda dialog:dialog.dismiss())
        page.locator("#clearAll").click()
        assert len(page.evaluate("window.e2Debug.regions"))==1,"cancel must keep masks"
        page.once("dialog",lambda dialog:dialog.accept())
        page.locator("#clearAll").click()
        assert len(page.evaluate("window.e2Debug.regions"))==0
        assert page.locator("#check").is_disabled()
        page.keyboard.press("Control+z")
        assert len(page.evaluate("window.e2Debug.regions"))==1
        results.append({"case":"100% zoom / fit / clear-all cancel-confirm / undo restores mask"})
        rect=page.evaluate("window.e2Debug.regions[0]")
        page.locator("#check").click()
        wait(page,"window.e2Debug.stage==='review' && window.e2Debug.outputBytes>0")
        with page.expect_download() as download:
            page.locator("#download").click()
        dest=OUT/"v1-local-png-redacted.png"
        download.value.save_as(dest)
        with Image.open(dest) as im:
            assert im.format=="PNG" and im.size==(640,400)
            pixels=im.convert("RGBA")
            x0,y0,x1,y1=[int(rect[k]) for k in ("x0","y0","x1","y1")]
            assert x1>x0 and y1>y0
            for x in range(x0,x1):
                for y in range(y0,y1):
                    assert pixels.getpixel((x,y))==(17,24,39,255),(x,y)
            assert pixels.getpixel((10,10))==(235,242,250,255)
            assert not im.info.get("exif")
        results.append({"case":"arbitrary local PNG via picker / undo / redo / real WASM output",
                        "source":"synthetic generated in CI, NOT one of the four fixture hashes",
                        "png_sha256":hashlib.sha256(dest.read_bytes()).hexdigest(),
                        "masked_rect":[x0,y0,x1,y1]})
        page.locator("#back").click()
        assert page.evaluate("window.e2Debug.stage==='edit'")
        # Invalid input must not destroy the current valid source or masks.
        fake=OUT/"not-really-an-image.png"
        fake.write_text("not a PNG even if extension claims PNG")
        page.locator("#pick").set_input_files(str(fake))
        wait(page,"document.getElementById('status').textContent.includes('导入失败')")
        assert page.evaluate("window.e2Debug.stage==='edit'")
        assert len(page.evaluate("window.e2Debug.regions"))==1
        assert page.evaluate("window.e2Debug.sourceSize")==[640,400]
        results.append({"case":"invalid image rejection preserves active edit"})

        # Distinct JPEG with synthetic EXIF orientation 6 is not a preset
        # fixture; Rust preview and redaction must agree on 50x80 dimensions.
        jpeg=make_exif_jpeg()
        page.once("dialog",lambda dialog:dialog.accept())
        page.locator("#pick").set_input_files(str(jpeg))
        wait(page,"window.e2Debug.stage==='edit' && window.e2Debug.sourceSize[0]===50")
        assert page.evaluate("window.e2Debug.sourceSize")==[50,80]
        page.locator("#createMask").click()
        page.locator("#check").click()
        wait(page,"window.e2Debug.stage==='review' && window.e2Debug.outputBytes>0")
        with page.expect_download() as download:
            page.locator("#download").click()
        jpeg_result=OUT/"v1-local-jpeg-exif-redacted.png"
        download.value.save_as(jpeg_result)
        with Image.open(jpeg_result) as im:
            assert im.format=="PNG" and im.size==(50,80)
            assert "exif" not in im.info
            assert b"PRIVATE-SYNTHETIC-DESCRIPTION" not in jpeg_result.read_bytes()
        results.append({"case":"arbitrary local EXIF JPEG / normalized preview / new metadata-free PNG",
                        "png_sha256":hashlib.sha256(jpeg_result.read_bytes()).hexdigest()})

        # Real browser's Clipboard read->trusted paste flows through the same
        # user-facing import path: the source changes to an unrelated PNG.
        page.locator("#back").click()
        page.once("dialog",lambda dialog:dialog.accept())
        payload=png.read_bytes()
        context.grant_permissions(["clipboard-read","clipboard-write"],origin=origin)
        page.evaluate("""async data => {
          const bytes=new Uint8Array(data);
          await navigator.clipboard.write([new ClipboardItem({
            'image/png':new Blob([bytes],{type:'image/png'})
          })]);
        }""",list(payload))
        page.keyboard.press("Control+v")
        wait(page,"window.e2Debug.stage==='edit' && window.e2Debug.sourceSize[0]===640")
        assert len(page.evaluate("window.e2Debug.regions"))==0
        results.append({"case":"trusted Chromium Ctrl+V imports arbitrary clipboard PNG"})

        assert not errors,errors
        assert all(u.startswith((origin+"/","blob:"+origin+"/")) for u in outgoing),outgoing
        page.screenshot(path=str(OUT/"v1-local-workbench.png"))
        (OUT/"v1-results.json").write_text(json.dumps({
            "environment":"actual Google Chrome, local Rust/WASM and synthetic-only test data",
            "results":results,"remote_requests":0,"page_errors":[]},ensure_ascii=False,indent=2)+"\n")
        context.close()
    finally:
        browser.close()
        server.shutdown()
print("SHAREREADY_V1_LOCAL_EDITOR_ACTUAL_CHROME_PASS",json.dumps(results,ensure_ascii=False))
