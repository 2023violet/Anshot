#!/usr/bin/env python3
"""E2.2: keyboard mask nudge/resize, CSS page scaling + DPR2, exact Rust PNG."""
import hashlib
import http.server
import json
import threading
from pathlib import Path
from PIL import Image
from playwright.sync_api import sync_playwright


# CSP-friendly test polling: Playwright wait_for_function uses page-side eval,
# which a correct strict script-src policy must block. CDP evaluate of this
# trusted direct function avoids relaxing the production/script CSP.
import time
def wait_csp(page, expression, arg=None, timeout=30000):
    until=time.monotonic()+timeout/1000
    while time.monotonic()<until:
        if page.evaluate(f"(value) => Boolean({expression})", arg):
            return
        page.wait_for_timeout(100)
    raise AssertionError("CSP-safe browser condition timeout: "+expression)

ROOT=Path(__file__).resolve().parents[2]
FIX=ROOT/"docs/research/fixtures/T01_CHAT.png"
OUT=Path("/tmp/e2-browser-evidence")
OUT.mkdir(exist_ok=True,parents=True)

class Handler(http.server.SimpleHTTPRequestHandler):
 def __init__(self,*args,**kwargs):
  super().__init__(*args,directory=str(ROOT),**kwargs)
 def log_message(self,*args):pass

server=http.server.ThreadingHTTPServer(("127.0.0.1",0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
records=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(channel="chrome",headless=True,args=["--no-sandbox"])
 try:
  for width,height,csszoom in [(1440,900,1.25),(1024,768,1.5),(800,820,1.0)]:
   ctx=browser.new_context(viewport={"width":width,"height":height},device_scale_factor=2,accept_downloads=True)
   page=ctx.new_page()
   errors=[];network=[]
   page.on("pageerror",lambda x:errors.append(str(x)))
   page.on("request",lambda r:network.append((r.method,r.url)))
   page.goto(f"http://127.0.0.1:{server.server_port}/e2-workbench/index.html")
   wait_csp(page, "window.e2Debug?.ready===true")
   page.locator("#pick").set_input_files(str(FIX))
   wait_csp(page, "window.e2Debug.stage==='edit'")
   page.evaluate("(z)=>document.documentElement.style.zoom=z",f"{int(csszoom*100)}%")
   page.locator("#frame").scroll_into_view_if_needed()
   box=page.locator("#frame").bounding_box()
   assert box,(width,csszoom)
   sourceW,sourceH=page.evaluate("window.e2Debug.sourceSize")
   scaleX=box["width"]/sourceW;scaleY=box["height"]/sourceH
   assert scaleX>0 and scaleY>0
   # Use actual DOM pixel scale, NOT editor zoom, as pointer coordinates.
   x0,y0,x1,y1=170,310,350,370
   page.mouse.move(box["x"]+(x0+.20)*scaleX,box["y"]+(y0+.20)*scaleY)
   page.mouse.down()
   page.mouse.move(box["x"]+(x1-.20)*scaleX,box["y"]+(y1-.20)*scaleY,steps=8)
   page.mouse.up()
   regions=page.evaluate("window.e2Debug.regions")
   assert len(regions)==1,(width,regions)
   r=regions[0]
   assert abs(min(r["x0"],r["x1"])-x0)<1.2,(width,r)
   assert abs(min(r["y0"],r["y1"])-y0)<1.2,(width,r)
   assert abs(max(r["x0"],r["x1"])-x1)<1.2,(width,r)
   assert abs(max(r["y0"],r["y1"])-y1)<1.2,(width,r)
   if width==800:
    page.locator(".mask.selected").focus()
   else:
    page.locator("#regionList .region").first.click()
   current=page.evaluate("window.e2Debug.regions")[0]
   page.keyboard.press("ArrowRight")
   moved=page.evaluate("window.e2Debug.regions")[0]
   assert min(moved["x0"],moved["x1"])==int(min(current["x0"],current["x1"]))+1,(width,moved,current)
   assert page.evaluate("window.e2Debug.outputRevision")==-1
   page.keyboard.press("Shift+ArrowDown")
   resized=page.evaluate("window.e2Debug.regions")[0]
   assert max(resized["y0"],resized["y1"])==int(__import__("math").ceil(max(moved["y0"],moved["y1"])))+1,(width,moved,resized)
   page.keyboard.press("Control+z")
   assert page.evaluate("window.e2Debug.regions")[0]==moved,("resize undo",width)
   page.keyboard.press("Control+Shift+z")
   assert page.evaluate("window.e2Debug.regions")[0]==resized,("resize redo",width)
   page.locator("#check").click()
   wait_csp(page, "window.e2Debug.stage==='review' && window.e2Debug.outputBytes>0")
   assert page.locator("#outputImage").evaluate("(img)=>img.complete && img.naturalWidth>0")
   with page.expect_download() as down:page.locator("#download").click()
   dest=OUT/f"T01_keyboard_{width}_dpr2.png"
   down.value.save_as(dest)
   with Image.open(dest) as out:
    assert out.format=="PNG"
    assert out.size==(sourceW,sourceH)
    pix=out.convert("RGBA")
    final=page.evaluate("window.e2Debug.regions")[0]
    l=int(__import__("math").floor(min(final["x0"],final["x1"])))
    t=int(__import__("math").floor(min(final["y0"],final["y1"])))
    rr=int(__import__("math").ceil(max(final["x0"],final["x1"])))
    bb=int(__import__("math").ceil(max(final["y0"],final["y1"])))
    for x,y in [(l,t),(rr-1,t),(l,bb-1),(rr-1,bb-1)]:
     assert pix.getpixel((x,y))==(17,24,39,255),(width,(x,y),pix.getpixel((x,y)))
    assert pix.getpixel((20,20))!=(17,24,39,255)
   assert not errors,(width,errors)
   assert all(method in ("GET","HEAD") and url.startswith(("http://127.0.0.1:","blob:http://127.0.0.1:")) for method,url in network),(width,network)
   page.screenshot(path=str(OUT/f"e2_keyboard_{width}_dpr2.png"))
   record={"width":width,"height":height,"DPR":2,"css_page_zoom":csszoom,
           "actual_dom_scale":[round(scaleX,6),round(scaleY,6)],
           "mask":final,"download_sha256":hashlib.sha256(dest.read_bytes()).hexdigest(),
           "keyboard_nudge_px":1,"keyboard_resize_px":1,"undo_redo":True,
           "remote_request_count":0}
   records.append(record)
   print("E2_PRECISION_ACCESSIBILITY_BROWSER_PASS",json.dumps(record,ensure_ascii=False))
   ctx.close()
 finally:
  browser.close()
  server.shutdown()
assert len(records)==3
(OUT/"e2-precision-results.json").write_text(json.dumps({"conditions":"real Chromium, three layouts, DPR2 and CSS zoom, synthetic-only","results":records,"scope":"engineering interaction verification, not cross-browser certification or E1e review"},ensure_ascii=False,indent=2)+"\n")
print("E2_KEYBOARD_SOURCE_PIXEL_DPR2_ZOOM_INVARIANT_PASS")
