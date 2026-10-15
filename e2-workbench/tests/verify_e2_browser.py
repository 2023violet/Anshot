#!/usr/bin/env python3
"""E2 real Chromium workbench test; only frozen synthetic fixtures and actual Rust WASM PNGs."""
import hashlib
import http.server
import io
import json
import threading
from pathlib import Path
from PIL import Image, ImageChops
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[2]
FIX=ROOT/"docs/research/fixtures"
OUT=Path("/tmp/e2-browser-evidence")
OUT.mkdir(parents=True,exist_ok=True)
MANIFEST={item["id"]:item for item in json.loads((FIX/"manifest.json").read_text())["fixtures"]}

class Handler(http.server.SimpleHTTPRequestHandler):
 def __init__(self,*args,**kwargs):
  super().__init__(*args,directory=str(ROOT),**kwargs)
 def log_message(self,*args):pass
 def end_headers(self):
  if self.path.startswith("/e2-workbench/index.html?deny_clipboard=1"):
   self.send_header("Permissions-Policy","clipboard-write=(), clipboard-read=()")
  super().end_headers()

server=http.server.ThreadingHTTPServer(("127.0.0.1",0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
summary=[]

def verify(actual_path,fixture):
 assert actual_path.read_bytes().startswith(b"\x89PNG\r\n\x1a\n"),"output must be PNG"
 with Image.open(actual_path) as pic, Image.open(FIX/fixture["file"]) as source:
  assert pic.format=="PNG"
  output=pic.convert("RGBA")
  original=source.convert("RGBA")
  assert output.size==(fixture["width"],fixture["height"]), ("dimensions",output.size)
  for region in fixture["sensitive_regions"]:
   x0,y0,x1,y1=region["rect_px"]
   crop=output.crop((x0,y0,x1,y1))
   expected=Image.new("RGBA",(x1-x0,y1-y0),(17,24,39,255))
   assert crop.tobytes()==expected.tobytes(),("pixel not masked",fixture["id"],region["id"])
  for keep in fixture["must_preserve_regions"]:
   box=tuple(keep["rect_px"])
   if fixture["id"]=="T05_JPEG_EXIF":
    delta=ImageChops.difference(output.crop(box).convert("RGB"),original.crop(box).convert("RGB"))
    assert max(pair[1] for pair in delta.getextrema())<=5,("JPEG cross-decoder",keep["id"])
   else:
    assert output.crop(box).tobytes()==original.crop(box).tobytes(),("keep changed",keep["id"])
  assert "exif" not in pic.info and b"eXIf" not in actual_path.read_bytes()
 return hashlib.sha256(actual_path.read_bytes()).hexdigest()

with sync_playwright() as pw:
 browser=pw.chromium.launch(channel="chrome",headless=True,args=["--no-sandbox"])
 try:
  for name in ("T01_CHAT","T02_ORDER","T04_EDGES","T05_JPEG_EXIF"):
   fixture=MANIFEST[name]
   ctx=browser.new_context(accept_downloads=True,viewport={"width":1440,"height":900})
   page=ctx.new_page()
   errors=[];requests=[]
   page.on("pageerror",lambda err:errors.append(str(err)))
   page.on("request",lambda req:requests.append((req.method,req.url)))
   page.goto("http://127.0.0.1:"+str(server.server_port)+"/e2-workbench/index.html")
   page.wait_for_function("window.e2Debug?.ready===true")
   # Unknown or tampered images are forbidden for this unreviewed integration build.
   page.locator("#pick").set_input_files({"name":"private_example.png","mimeType":"image/png","buffer":b"unknown fake private PNG"})
   page.wait_for_function("document.getElementById('status').textContent.includes('只能使用冻结')")
   assert page.evaluate("window.e2Debug.stage")=="empty"
   assert page.locator("#check").is_disabled()
   page.locator("#pick").set_input_files(str(FIX/fixture["file"]))
   page.wait_for_function("window.e2Debug?.stage==='edit'")
   assert page.evaluate("window.e2Debug.fixture")==name
   assert tuple(page.evaluate("window.e2Debug.sourceSize"))==(fixture["width"],fixture["height"])
   page.locator("#check").is_disabled()
   frame=page.locator("#frame")
   box=frame.bounding_box()
   factor=page.evaluate("window.e2Debug.zoom")
   assert box and 0<factor<=1,(name,box,factor)
   page.locator("#maskTool").click()
   for region in fixture["sensitive_regions"]:
    x0,y0,x1,y1=region["rect_px"]
    start=(box["x"]+(x0+0.18)*factor,box["y"]+(y0+0.18)*factor)
    stop=(box["x"]+(x1-0.18)*factor,box["y"]+(y1-0.18)*factor)
    page.mouse.move(*start);page.mouse.down()
    page.mouse.move(*stop,steps=7);page.mouse.up()
   count=len(fixture["sensitive_regions"])
   assert len(page.evaluate("window.e2Debug.regions"))==count,("not all masks created",name)
   assert page.locator("#count").inner_text()==str(count)
   assert page.locator("#regionList .region").count()==count
   page.keyboard.press("Control+z")
   assert len(page.evaluate("window.e2Debug.regions"))==count-1
   page.keyboard.press("Control+Shift+z")
   assert len(page.evaluate("window.e2Debug.regions"))==count
   # Move a selected mask through the real pointer handlers and undo the move.
   pre_edit=page.evaluate("window.e2Debug.regions")
   page.locator("#regionList .region").first.click()
   chosen=page.locator(".mask.selected").bounding_box()
   assert chosen and chosen["width"]>10 and chosen["height"]>10
   mx=chosen["x"]+chosen["width"]*.5;my=chosen["y"]+chosen["height"]*.5
   page.mouse.move(mx,my);page.mouse.down()
   page.mouse.move(mx+9,my+9,steps=5);page.mouse.up()
   assert page.evaluate("window.e2Debug.regions")!=pre_edit,("mask move did not modify geometry",name)
   page.keyboard.press("Control+z")
   assert page.evaluate("window.e2Debug.regions")==pre_edit,("mask move undo failed",name)
   # Use an actual bottom-right handle for resizing, then undo.
   page.locator("#regionList .region").first.click()
   grip=page.locator(".mask.selected .handle").bounding_box()
   assert grip
   gx=grip["x"]+grip["width"]*.5;gy=grip["y"]+grip["height"]*.5
   page.mouse.move(gx,gy);page.mouse.down()
   page.mouse.move(gx+8,gy+7,steps=5);page.mouse.up()
   assert page.evaluate("window.e2Debug.regions")!=pre_edit,("mask resize did not modify geometry",name)
   page.keyboard.press("Control+z")
   assert page.evaluate("window.e2Debug.regions")==pre_edit,("mask resize undo failed",name)
   z_before=page.evaluate("window.e2Debug.zoom")
   page.locator("#zoomIn").click()
   assert page.evaluate("window.e2Debug.zoom")>z_before
   page.locator("#zoomFit").click()
   assert abs(page.evaluate("window.e2Debug.zoom")-z_before)<.00001
   page.locator("#maskTool").click()
   page.locator("#check").click()
   page.wait_for_function("window.e2Debug?.stage==='review' && window.e2Debug.outputBytes>0")
   assert page.locator("#outputImage").evaluate("(img)=>img.complete && img.naturalWidth>0")
   assert not page.locator("#copy").is_disabled()
   assert not page.locator("#download").is_disabled()
   with page.expect_download() as download:page.locator("#download").click()
   dest=OUT/(name+"_e2.png")
   download.value.save_as(dest)
   hash_=verify(dest,fixture)
   page.screenshot(path=str(OUT/(name+"_review.png")))
   assert not errors,(name,errors)
   assert requests and all(method in ("GET","HEAD") and (url.startswith("http://127.0.0.1:") or url.startswith("blob:http://127.0.0.1:")) for method,url in requests),(name,requests)
   page.locator("#back").click()
   assert page.evaluate("window.e2Debug.stage")=="edit"
   # An actual new edit invalidates prior encoded output and disables reuse.
   page.locator("#regionList .region").first.click()
   page.locator("#delete").click()
   assert page.evaluate("window.e2Debug.outputRevision")==-1
   assert page.evaluate("window.e2Debug.outputBytes")==0
   page.keyboard.press("Control+z")
   assert len(page.evaluate("window.e2Debug.regions"))==count
   summary.append({"fixture":name,"masks":count,"png_sha256":hash_,"png_bytes":dest.stat().st_size,
                   "zoom":factor,"review_source":"decoded real Rust/WASM PNG bytes",
                   "stale_export_rejected":True,"remote_requests":0})
   print("E2_REAL_WORKBENCH_FIXTURE_PASS",json.dumps(summary[-1],ensure_ascii=False))
   ctx.close()
  # Independent permissions-policy failure case, preserve the same encoded download.
  fixture=MANIFEST["T01_CHAT"]
  ctx=browser.new_context(accept_downloads=True,viewport={"width":1440,"height":900})
  page=ctx.new_page()
  page.goto("http://127.0.0.1:"+str(server.server_port)+"/e2-workbench/index.html?deny_clipboard=1")
  page.wait_for_function("window.e2Debug?.ready===true")
  page.locator("#pick").set_input_files(str(FIX/fixture["file"]))
  page.wait_for_function("window.e2Debug.stage==='edit'")
  box=page.locator("#frame").bounding_box();z=page.evaluate("window.e2Debug.zoom")
  rect=fixture["sensitive_regions"][0]["rect_px"]
  x0,y0,x1,y1=rect
  page.mouse.move(box["x"]+(x0+.2)*z,box["y"]+(y0+.2)*z);page.mouse.down()
  page.mouse.move(box["x"]+(x1-.2)*z,box["y"]+(y1-.2)*z,steps=6);page.mouse.up()
  page.locator("#check").click()
  page.wait_for_function("window.e2Debug.stage==='review'")
  page.locator("#copy").click()
  page.wait_for_function("document.getElementById('status').textContent.includes('复制失败')")
  assert page.locator("#download").is_enabled()
  print("E2_REAL_CLIPBOARD_DENIAL_AND_DOWNLOAD_FALLBACK_PASS")
  ctx.close()
 finally:
  browser.close()
  server.shutdown()

assert len(summary)==4
(OUT/"e2-results.json").write_text(json.dumps({
 "results":summary,"conditions":"actual Chromium, Rust WASM rebuilt from E2 source, frozen synthetic input only",
 "security":"not an independent E1e audit; real private input prohibited","clipboard":"denied and fallback verified, success readback not certified"
},ensure_ascii=False,indent=2)+"\n")
print("E2_REAL_WORKBENCH_FOUR_PNGS_EDIT_REVIEW_STALE_NEGATIVE_NETWORK_PASS")
