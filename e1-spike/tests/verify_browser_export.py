#!/usr/bin/env python3
"""E1 actual Rust/WASM Chromium synthetic export, downloaded binary verified against frozen manifest."""
import hashlib,http.server,json,os,threading,time
from pathlib import Path
from playwright.sync_api import sync_playwright
from PIL import Image,ImageChops
REPO=Path(__file__).resolve().parents[2]
FIX=REPO/"docs/research/fixtures"
OUT=Path("/tmp/e1-web-browser");OUT.mkdir(exist_ok=True)
manifest={x["id"]:x for x in json.loads((FIX/"manifest.json").read_text())["fixtures"]}
class Handler(http.server.SimpleHTTPRequestHandler):
 def __init__(self,*args,**kwargs):super().__init__(*args,directory=str(REPO),**kwargs)
 def log_message(self,*a):pass
server=http.server.ThreadingHTTPServer(("127.0.0.1",0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
results=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(channel="chrome",headless=True,args=["--no-sandbox"])
 try:
  for name in ["T01_CHAT","T02_ORDER","T04_EDGES","T05_JPEG_EXIF"]:
   spec=manifest[name]
   ctx=browser.new_context(accept_downloads=True,viewport={"width":1280,"height":900})
   page=ctx.new_page()
   network=[]
   page.on("request",lambda r:network.append({"method":r.method,"url":r.url}))
   errors=[]
   page.on("pageerror",lambda e:errors.append(str(e)))
   page.goto(f"http://127.0.0.1:{server.server_port}/e1-spike/web/index.html",wait_until="load")
   page.wait_for_function("window.e1Debug?.ready===true",timeout=45000)
   page.locator("#profile").select_option(name)
   page.locator("#file").set_input_files(str(FIX/spec["file"]))
   page.locator("#generate").click()
   page.locator("#download").wait_for(state="visible")
   page.wait_for_function("window.e1Debug?.outputBytes>0 && !document.getElementById('download').disabled",timeout=45000)
   with page.expect_download(timeout=20000) as ev:page.locator("#download").click()
   file=OUT/(name+"_wasm.png")
   ev.value.save_as(file)
   assert not errors,errors
   with Image.open(file) as result, Image.open(FIX/spec["file"]) as original:
    assert result.format=="PNG"
    if original.format=="JPEG":orig=original.convert("RGBA")
    else:orig=original.convert("RGBA")
    image=result.convert("RGBA")
    assert image.size==(spec["width"],spec["height"]),(name,image.size)
    for reg in spec["sensitive_regions"]:
     x0,y0,x1,y1=reg["rect_px"]
     assert image.getpixel((x0,y0))==(17,24,39,255),(name,reg["id"])
     assert image.getpixel((x1-1,y1-1))==(17,24,39,255),(name,reg["id"])
     pixels=Image.new("RGBA",(x1-x0,y1-y0),(17,24,39,255))
     assert image.crop((x0,y0,x1,y1)).tobytes()==pixels.tobytes(),(name,reg["id"],"pixel leak")
    for keep in spec["must_preserve_regions"]:
     bb=tuple(keep["rect_px"])
     if name=="T05_JPEG_EXIF":
      # Rust image/zune-jpeg and Pillow/libjpeg use distinct IDCT rounding;
      # exact source-pixel preservation is separately proved inside the Rust codec test.
      diff=ImageChops.difference(image.crop(bb).convert("RGB"),orig.crop(bb).convert("RGB"))
      maximum=max(extreme[1] for extreme in diff.getextrema())
      print("E1_JPEG_DIFFERENT_DECODER_PRESERVE_MAX_DELTA",maximum)
      assert maximum<=5,(name,keep["id"],"cross-decoder difference too large",maximum)
     else:
      assert image.crop(bb).tobytes()==orig.crop(bb).tobytes(),(name,keep["id"])
    assert "exif" not in result.info and b"eXIf" not in file.read_bytes()
   assert all(x["method"] in ("GET","HEAD") and (x["url"].startswith("http://127.0.0.1:") or x["url"].startswith("blob:http://127.0.0.1:")) for x in network),network
   page.screenshot(path=str(OUT/(name+"_review.png")))
   metrics=page.evaluate("window.e1Debug")
   results.append({"fixture":name,"png_sha256":hashlib.sha256(file.read_bytes()).hexdigest(),"png_bytes":file.stat().st_size,
                   "render_ms":metrics["renderMs"],"masks":metrics["masks"],"network":network,"page_errors":errors})
   print("E1_WASM_REAL_EXPORT_PASS",name,json.dumps(results[-1]))
   ctx.close()
 finally:
  browser.close();server.shutdown()
(OUT/"browser-evidence.json").write_text(json.dumps({"results":results,"environment":"Chrome Playwright 1.55, real local WASM","safety_scope":"synthetic only, no human privacy guarantee"},indent=2)+"\n")
assert len(results)==4
print("E1_FOUR_WASM_DOWNLOADED_PNGS_REAL_PIXEL_AND_NETWORK_CHECK_PASS")
