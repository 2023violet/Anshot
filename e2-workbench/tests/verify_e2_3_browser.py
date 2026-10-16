#!/usr/bin/env python3
"""E2.3 actual Chromium keyboard, CDP visual page scale and Clipboard paste-back.

Only SHA256-frozen synthetic T01; this is NOT a privacy/security certification.
"""
import hashlib
import http.server
import json
import threading
from pathlib import Path

from PIL import Image
from playwright.sync_api import TimeoutError as PlaywrightTimeout, sync_playwright

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "docs/research/fixtures/T01_CHAT.png"
OUT = Path("/tmp/e2-browser-evidence")
OUT.mkdir(parents=True, exist_ok=True)


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def log_message(self, *args):
        pass


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
origin = f"http://127.0.0.1:{server.server_port}"
observations = []

with sync_playwright() as pw:
    browser = pw.chromium.launch(channel="chrome", headless=True, args=["--no-sandbox"])
    try:
        for width, height, page_scale in ((1440, 900, 1.25), (1024, 768, 1.5), (800, 820, 1.2)):
            ctx = browser.new_context(viewport={"width": width, "height": height},
                                      device_scale_factor=2, accept_downloads=True)
            page = ctx.new_page()
            failures, requests = [], []
            page.on("pageerror", lambda error: failures.append(str(error)))
            page.on("request", lambda req: requests.append((req.method, req.url)))
            page.goto(origin + "/e2-workbench/index.html")
            page.wait_for_function("window.e2Debug?.ready===true")
            page.locator("#pick").set_input_files(str(FIXTURE))
            page.wait_for_function("window.e2Debug.stage==='edit'")

            cdp = ctx.new_cdp_session(page)
            # Real Chromium visual viewport scaling (CDP PageScaleFactor), not CSS zoom.
            cdp.send("Emulation.setPageScaleFactor", {"pageScaleFactor": page_scale})
            page.wait_for_function("(scale)=>Math.abs(window.visualViewport.scale-scale)<0.05",
                                   arg=page_scale)
            actual_scale = page.evaluate("window.visualViewport.scale")

            page.locator("#createMask").focus()
            page.keyboard.press("Enter")
            assert len(page.evaluate("window.e2Debug.regions")) == 1
            assert page.evaluate("document.activeElement?.classList.contains('mask')") is True
            before = page.evaluate("window.e2Debug.regions")[0]
            page.keyboard.press("ArrowRight")
            moved = page.evaluate("window.e2Debug.regions")[0]
            assert moved["x0"] == before["x0"] + 1, (width, moved, before)
            assert page.evaluate("document.activeElement?.classList.contains('mask')") is True
            page.keyboard.press("Shift+ArrowDown")
            resized = page.evaluate("window.e2Debug.regions")[0]
            assert resized["y1"] == moved["y1"] + 1, (width, resized, moved)
            assert page.evaluate("document.activeElement?.classList.contains('mask')") is True
            assert "已使用键盘" in page.locator("#status").inner_text()
            page.keyboard.press("Control+z")
            assert page.evaluate("window.e2Debug.regions")[0] == moved
            page.keyboard.press("Control+Shift+z")
            assert page.evaluate("window.e2Debug.regions")[0] == resized
            page.keyboard.press("n")
            assert len(page.evaluate("window.e2Debug.regions")) == 2
            assert page.evaluate("document.activeElement?.dataset.id") == "2"
            assert page.locator("#check").is_enabled()
            assert page.locator("#keyboardHelp").count() == 1

            # Actually drive pointer geometry while Chromium's real visual
            # viewport is scaled; keyboard-only paths cannot prove pointer math.
            page.locator("#maskTool").click()
            frame = page.locator("#frame")
            frame.scroll_into_view_if_needed()
            bounds = frame.bounding_box()
            assert bounds
            image_w, image_h = page.evaluate("window.e2Debug.sourceSize")
            sx, sy = bounds["width"] / image_w, bounds["height"] / image_h
            x0, y0, x1, y1 = 28, 35, 56, 65
            page.mouse.move(bounds["x"] + (x0 + 0.25) * sx,
                            bounds["y"] + (y0 + 0.25) * sy)
            page.mouse.down()
            page.mouse.move(bounds["x"] + (x1 - 0.25) * sx,
                            bounds["y"] + (y1 - 0.25) * sy, steps=7)
            page.mouse.up()
            scaled = page.evaluate("window.e2Debug.regions")
            assert len(scaled) == 3, (width, scaled)
            pointer_region = scaled[-1]
            for actual, expected in zip(
                (pointer_region["x0"], pointer_region["y0"],
                 pointer_region["x1"], pointer_region["y1"]),
                (x0, y0, x1, y1),
            ):
                assert abs(actual - expected) < 1.5, (
                    "native visual viewport pointer mapping", width,
                    actual_scale, pointer_region
                )

            page.locator("#check").click()
            page.wait_for_function("window.e2Debug.stage==='review' && window.e2Debug.outputBytes>0")
            assert page.evaluate("document.activeElement?.id") == "back"
            assert page.evaluate("document.querySelector('.app').inert") is True
            with page.expect_download() as event:
                page.locator("#download").click()
            destination = OUT / f"e2_3_keyboard_{width}.png"
            event.value.save_as(destination)
            with Image.open(destination) as image:
                pixels = image.convert("RGBA")
                assert pixels.size == (1200, 800)
                for region in page.evaluate("window.e2Debug.regions"):
                    corners = ((region["x0"], region["y0"]),
                               (region["x1"]-1, region["y1"]-1))
                    for x, y in corners:
                        assert pixels.getpixel((int(x), int(y))) == (17, 24, 39, 255)
                assert pixels.getpixel((10, 10)) != (17, 24, 39, 255)
            page.locator("#back").click()
            assert page.evaluate("document.querySelector('.app').inert") is False
            assert page.evaluate("document.activeElement?.id") == "check"
            assert all(method in ("GET", "HEAD") and
                       url.startswith((origin + "/", "blob:" + origin + "/"))
                       for method, url in requests), requests
            assert not failures, (width, failures)
            page.screenshot(path=str(OUT / f"e2_3_keyboard_{width}.png"))
            observations.append({"viewport": [width, height], "dpr": 2,
                                 "chromium_visual_viewport_scale": actual_scale,
                                 "keyboard_created": 2, "pointer_created": 1, "focus_restored": True,
                                 "png_sha256": hashlib.sha256(destination.read_bytes()).hexdigest(),
                                 "no_remote_requests": True})
            ctx.close()

        # Real Clipboard write then trusted browser paste event. No simulated clipboard data.
        ctx = browser.new_context(viewport={"width": 1440, "height": 900}, accept_downloads=True)
        ctx.grant_permissions(["clipboard-read", "clipboard-write"], origin=origin)
        page = ctx.new_page()
        page.goto(origin + "/e2-workbench/index.html")
        page.wait_for_function("window.e2Debug?.ready===true")
        page.locator("#pick").set_input_files(str(FIXTURE))
        page.wait_for_function("window.e2Debug.stage==='edit'")
        page.locator("#createMask").click()
        page.locator("#check").click()
        page.wait_for_function("window.e2Debug.stage==='review'")
        page.locator("#copy").click()
        page.wait_for_function("window.e2Debug.copyState!=='not_attempted'")
        copy_state = page.evaluate("window.e2Debug.copyState")
        assert copy_state == "write_succeeded_readback_unverified", copy_state
        page.evaluate("""() => {
          window.__e23Paste={state:'pending'};
          // Trusted paste is browser-defined for an editable target. Focusing
          // a regular button can silently suppress the actual paste event.
          const target=document.createElement('div');
          target.id='e23pasteTarget';target.contentEditable='true';
          target.setAttribute('aria-label','Synthetic PNG paste verification');
          target.style.cssText='position:fixed;left:8px;top:8px;width:2px;height:2px;opacity:.01;z-index:10000';
          document.body.append(target);
          const rgbaHash=async blob=>{
            const bitmap=await createImageBitmap(blob);
            const canvas=document.createElement('canvas');
            canvas.width=bitmap.width;canvas.height=bitmap.height;
            const c=canvas.getContext('2d',{willReadFrequently:true});
            c.drawImage(bitmap,0,0);bitmap.close();
            const data=c.getImageData(0,0,canvas.width,canvas.height).data;
            const digest=await crypto.subtle.digest('SHA-256',data);
            return [...new Uint8Array(digest)].map(x=>x.toString(16).padStart(2,'0')).join('');
          };
          const expected=fetch(document.getElementById('outputImage').src)
            .then(r=>r.blob()).then(rgbaHash);
          document.addEventListener('paste',async e=>{
            e.preventDefault();e.stopImmediatePropagation();
            const item=[...(e.clipboardData?.items||[])].find(i=>i.type==='image/png');
            if(!item){window.__e23Paste={state:'no_png_in_real_paste'};return;}
            try{
              const file=item.getAsFile();
              window.__e23Paste={
                state:'done',matches:(await rgbaHash(file))===(await expected),
                image_png_type:file.type,actual_bytes:file.size
              };
            }catch(error){window.__e23Paste={state:'error',message:String(error)};}
          },{capture:true,once:true});
        }""")
        page.locator("#e23pasteTarget").focus()
        assert page.evaluate("document.activeElement?.id") == "e23pasteTarget"
        page.keyboard.press("Control+v")
        paste_result = None
        try:
            page.wait_for_function("window.__e23Paste?.state!=='pending'", timeout=7000)
            paste_result = page.evaluate("window.__e23Paste")
        except PlaywrightTimeout:
            paste_result = {"state": "real_paste_not_observed"}
        assert paste_result["state"] == "done" and paste_result["matches"], paste_result
        observations.append({"clipboard_write_state": copy_state,
                             "actual_trusted_paste": paste_result})
        print("E2_3_ACTUAL_BROWSER_CLIPBOARD_PASTE_RGBA_MATCH_PASS", json.dumps(paste_result))
        ctx.close()

        # Deterministic race: hold write promise, close review and edit, then resolve it.
        ctx = browser.new_context(viewport={"width": 1440, "height": 900})
        page = ctx.new_page()
        page.goto(origin + "/e2-workbench/index.html")
        page.wait_for_function("window.e2Debug?.ready===true")
        page.locator("#pick").set_input_files(str(FIXTURE))
        page.wait_for_function("window.e2Debug.stage==='edit'")
        page.locator("#createMask").click()
        page.locator("#check").click()
        page.wait_for_function("window.e2Debug.stage==='review'")
        page.evaluate("""() => {
          Object.defineProperty(navigator,'clipboard',{configurable:true,value:{
            write:()=>new Promise(resolve=>window.__finishDelayedCopy=resolve)
          }});
        }""")
        page.locator("#copy").click()
        page.wait_for_function("typeof window.__finishDelayedCopy==='function'")
        page.locator("#back").click()
        page.locator("#createMask").click()
        page.evaluate("window.__finishDelayedCopy()")
        page.wait_for_function("window.e2Debug.copyState==='stale_during_copy'")
        assert "旧复制结果不可作为当前成品" in page.locator("#status").inner_text()
        assert page.evaluate("window.e2Debug.outputRevision") == -1
        observations.append({"delayed_clipboard_write": "stale_during_copy",
                             "method": "deterministic delayed Promise, NOT native clipboard"})
        print("E2_3_CLIPBOARD_REVISION_RACE_NEGATIVE_PASS")
        # Replacement from the review state must dismiss the modal, restore
        # editor focus and invalidate old PNG. No old review overlay may remain.
        page.locator("#check").click()
        page.wait_for_function("window.e2Debug.stage==='review'")
        page.once("dialog", lambda dialog: dialog.accept())
        page.locator("#pick").set_input_files(str(FIXTURE))
        page.wait_for_function(
            "window.e2Debug.stage==='edit' && window.e2Debug.outputRevision===-1"
            " && window.e2Debug.regions.length===0"
        )
        assert not page.locator("#review").evaluate("(el)=>el.classList.contains('visible')")
        assert page.evaluate("document.querySelector('.app').inert") is False
        assert page.evaluate("document.activeElement?.id") == "createMask"
        assert page.locator("#check").is_disabled()
        observations.append({"replace_during_review": "modal_closed_focus_restored",
                             "old_export": "invalidated", "input": "synthetic_T01"})
        print("E2_3_REPLACE_FROM_REVIEW_RECOVERY_PASS")
        ctx.close()
    finally:
        browser.close()
        server.shutdown()

(OUT / "e2-3-results.json").write_text(json.dumps({
    "environment": "Actual headless Chrome, DPR2, native Chromium visual viewport page scale",
    "input": "SHA256-locked synthetic fixture T01 only",
    "not_claimed": "Independent E1e signoff, browser-menu zoom, all browsers, or real-user privacy safety",
    "results": observations,
}, ensure_ascii=False, indent=2) + "\n")
print("E2_3_KEYBOARD_FOCUS_ZOOM_REAL_PASTE_AND_REVISION_PASS")
