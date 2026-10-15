#!/usr/bin/env python3
"""S03: isolate whether native browser pointer release/overlay state affects a second synthetic mask.

This is a research-only automated diagnostic on a third-party site. Not a human
usability observation, product source, exploit test or vendor bug determination.
"""
import hashlib,json,platform,time
from pathlib import Path
from PIL import Image,ImageChops
from playwright.sync_api import sync_playwright

SRC=Path("docs/research/fixtures/T02_ORDER.png").resolve()
OUT=Path("/tmp/share-ready-s03-pointer-recovery");OUT.mkdir(parents=True,exist_ok=True)
REGIONS={"recipient":(105,469,568,518),"address":(105,604,679,654)}
VARIANTS=[
 ("baseline", "recipient", "address",900),
 ("neutral_click", "recipient", "address",900),
 ("escape", "recipient", "address",900),
 ("mouseleave", "recipient", "address",900),
 ("viewport_tall", "recipient", "address",1200),
 ("neutral_click_reverse", "address", "recipient",900),
 ("escape_reverse", "address", "recipient",900),
 ("viewport_tall_reverse", "address", "recipient",1200),
]
results=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(channel="chrome",headless=True,args=["--no-sandbox","--disable-dev-shm-usage"])
 for case,(variant,first,second,h) in enumerate(VARIANTS):
  ctx=browser.new_context(viewport={"width":1280,"height":h},locale="en-US",accept_downloads=True)
  page=ctx.new_page()
  record={"case":case,"variant":variant,"order":[first,second],"chrome_version":browser.version,
          "os":platform.platform(),"viewport":[1280,h],"status":"NOT_RUN","steps":[]}
  try:
   page.goto("https://redactiontools.com/redact-screenshot",wait_until="domcontentloaded",timeout=45000)
   page.locator("input[type=file]").first.set_input_files(str(SRC))
   page.wait_for_timeout(3800)
   canvas=page.locator("canvas").first
   assert canvas.count()==1,"canvas missing"
   canvas.scroll_into_view_if_needed()
   page.evaluate("""()=>{
      window.__evt=[];
      for(const el of [document,window]){
       for(const ty of ['pointerdown','pointerup','pointercancel','gotpointercapture','lostpointercapture','mousedown','mouseup','click']){
        el.addEventListener(ty,e=>{if(window.__evt.length>=250)window.__evt.shift();
          window.__evt.push({type:e.type,target:e.target?.tagName,phase:el===document?'doc':'window',
             x:Math.round(e.clientX||0),y:Math.round(e.clientY||0),buttons:e.buttons,pointerId:e.pointerId||null})},true)
       }
      }
   }""")
   def drag(region):
    box=canvas.bounding_box()
    assert box and box["width"]>300
    x1,y1,x2,y2=REGIONS[region]
    scale_x=box["width"]/1200;scale_y=box["height"]/800
    p1=(box["x"]+(x1+5)*scale_x,box["y"]+(y1+5)*scale_y)
    p2=(box["x"]+(x2-5)*scale_x,box["y"]+(y2-5)*scale_y)
    before=page.evaluate("""([x,y])=>{const e=document.elementFromPoint(x,y);
       return {element:e?.tagName,classes:String(e?.className||'').slice(0,130),
       overlayCount:document.querySelectorAll('button[title*="Drawn by you"]').length}}""",list(p1))
    page.mouse.move(*p1,steps=6);page.mouse.down();page.wait_for_timeout(90)
    page.mouse.move(*p2,steps=16);page.wait_for_timeout(110);page.mouse.up()
    page.wait_for_timeout(650)
    after=page.evaluate("""()=>({overlays:document.querySelectorAll('button[title*="Drawn by you"]').length,
       events:(window.__evt||[]).slice(-32)})""")
    screenshot=OUT/f"case{case}_{region}.jpg"
    screenshot.write_bytes(page.screenshot(type="jpeg",quality=40))
    record["steps"].append({"target":region,"from":p1,"to":p2,"bbox":box,"before":before,"after":after})
   drag(first)
   if variant.startswith("neutral_click"):
    # Neutral outside of the editor and all on-screen action buttons.
    page.mouse.move(1200,410);page.mouse.click(1200,410);page.wait_for_timeout(250)
   elif variant.startswith("escape"):
    page.keyboard.press("Escape");page.wait_for_timeout(250)
   elif variant=="mouseleave":
    page.mouse.move(60,410,steps=10);page.wait_for_timeout(400)
   drag(second)
   with page.expect_download(timeout=15000) as event:
    page.get_by_role("button",name="Download PNG",exact=True).click()
   dest=OUT/f"case{case}_export.png";event.value.save_as(dest)
   before=Image.open(SRC).convert("RGB")
   after=Image.open(dest).convert("RGB")
   assert before.size==after.size
   checks={}
   for key,bb in REGIONS.items():
    x1,y1,x2,y2=bb
    d=ImageChops.difference(before.crop(bb),after.crop(bb))
    checks[key]={"changed_pixels_in_rectangle":d.getbbox() is not None,
                 "center_rgb":after.getpixel(((x1+x2)//2,(y1+y2)//2))}
   record.update({"status":"REAL_EXPORT_CREATED","image_checks":checks,
                  "export_sha256":hashlib.sha256(dest.read_bytes()).hexdigest(),
                  "export_bytes":dest.stat().st_size})
  except Exception as e:
   record.update({"status":"BLOCKED","error":str(e)[:850]})
  finally:
   brief={"case":case,"variant":variant,"order":[first,second],"status":record["status"],
          "overlay_counts":[s["after"]["overlays"] for s in record["steps"]],
          "events_per_step":[[x["type"]+":"+x["phase"] for x in s["after"]["events"] if x["phase"]=="doc"][-14:] for s in record["steps"]],
          "target_centers":record.get("image_checks"),"error":record.get("error"),
          "chrome":browser.version}
   print("S03_RECOVERY_RESULT",json.dumps(brief,ensure_ascii=False))
   (OUT/f"case{case}_detail.json").write_text(json.dumps(record,ensure_ascii=False,indent=2)+"\n",encoding="utf8")
   results.append(brief)
   page.close();ctx.close()
 browser.close()
(OUT/"summary.json").write_text(json.dumps({"schema":1,"research_only":True,"human_samples":0,
  "cases":results},indent=2,ensure_ascii=False)+"\n",encoding="utf8")
print("S03_POINTER_RECOVERY_DONE; NO_PRODUCT_CODE; NO_HUMAN_CLAIMS")
