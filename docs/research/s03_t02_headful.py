#!/usr/bin/env python3
"""S03 forensic study of 2nd/3rd synthetic manual mask gestures. Research tooling only."""
import json,platform,time
from pathlib import Path
from playwright.sync_api import sync_playwright

F=Path("docs/research/fixtures/T02_ORDER.png").resolve()
O=Path("/tmp/s03-t02-xvfb-headful");O.mkdir(exist_ok=True,parents=True)
REGIONS={"recipient":(105,469,568,518),"address":(105,604,679,654)}
ORDERS=[["address","recipient"],["recipient","address"]]

def clip(x):return str(x)[:150]
def state(page,coords):
    return page.evaluate("""({px,py})=>{
       const el=document.elementFromPoint(px,py);
       const label=n=>!n?'NONE':(n.tagName+'#'+(n.id||'')+'.'+String(n.className?.baseVal||n.className||'').slice(0,120)+'['+(n.getAttribute('role')||'')+']');
       return {element:label(el),ancestor:label(el?.parentElement),ancestor2:label(el?.parentElement?.parentElement),
         pointerEvents:el?getComputedStyle(el).pointerEvents:null,
         elementAttr:el?el.outerHTML.slice(0,290):null,
         events:(window.__s03events||[]).slice(-10),
         canvas:document.querySelector('canvas')?label(document.querySelector('canvas')):null
       }
    }""",{"px":coords[0],"py":coords[1]})

with sync_playwright() as pw:
  b=pw.chromium.launch(channel="chrome",headless=False,args=["--no-sandbox","--disable-dev-shm-usage"])
  ctx=b.new_context(viewport={"width":1280,"height":900},locale="en-US",accept_downloads=True)
  for case,order in enumerate(ORDERS):
    page=ctx.new_page()
    entry={"case":case,"order":order,"pacing":"XVFB_VISIBLE_WINDOW","browser":b.version,"os":platform.platform(),"steps":[]}
    try:
      page.goto("https://redactiontools.com/redact-screenshot",wait_until="domcontentloaded",timeout=50000)
      page.locator("input[type=file]").first.set_input_files(str(F))
      page.wait_for_timeout(4200)
      canvas=page.locator("canvas").first
      canvas.scroll_into_view_if_needed()
      page.evaluate("""()=>{
        window.__s03events=[];
        for(const t of ['pointerdown','pointermove','pointerup','mousedown','mousemove','mouseup','click']){
          document.addEventListener(t,e=>{
            if(window.__s03events.length>450)window.__s03events.shift();
            if(t==='mousemove'||t==='pointermove')return;
            window.__s03events.push({t,tag:e.target?.tagName,id:e.target?.id,cls:String(e.target?.className?.baseVal||e.target?.className||'').slice(0,80),
              x:Math.round(e.clientX),y:Math.round(e.clientY),buttons:e.buttons,pointerType:e.pointerType||null,isTrusted:e.isTrusted});
          },true)
        }
      }""")
      for n,region in enumerate(order):
        bounds=canvas.bounding_box()
        x1,y1,x2,y2=REGIONS[region]
        sx=bounds["x"]+(x1+4)*bounds["width"]/1200
        sy=bounds["y"]+(y1+4)*bounds["height"]/800
        ex=bounds["x"]+(x2-4)*bounds["width"]/1200
        ey=bounds["y"]+(y2-4)*bounds["height"]/800
        step={"target":region,"bbox":bounds,"start":(sx,sy),"end":(ex,ey),
          "before":state(page,(sx,sy))}
        page.mouse.move(sx,sy)
        page.mouse.down()
        if case>=2:page.wait_for_timeout(220)
        page.mouse.move(ex,ey,steps=20)
        if case>=2:page.wait_for_timeout(240)
        page.mouse.up()
        page.wait_for_timeout(1200)
        txt=page.locator("body").inner_text()
        step["hidden_indicators"]=[x.strip() for x in txt.splitlines() if 'hidden' in x.lower()][:9]
        step["after_start"]=state(page,(sx,sy))
        step["after_end"]=state(page,(ex,ey))
        e=page.evaluate("()=>(window.__s03events||[]).slice(-12)")
        step["last_events"]=e
        ss=O/f"case{case}_{n}_{region}.jpg"
        ss.write_bytes(page.screenshot(type="jpeg",quality=48))
        entry["steps"].append(step)
      dl=page.get_by_role("button",name="Download PNG")
      with page.expect_download(timeout=12000) as d:dl.click()
      out=O/f"case{case}_export.png"
      d.value.save_as(out)
      entry["export_size"]=out.stat().st_size
      entry["status"]="EXPORTED"
    except Exception as exc:
      entry["status"]="BLOCKED"
      entry["error"]=str(exc)[:900]
    finally:
      print("S03_POINTER_DIAGNOSTIC",json.dumps(entry,ensure_ascii=False))
      (O/f"case{case}_events.json").write_text(json.dumps(entry,indent=2,ensure_ascii=False))
      page.close()
  ctx.close();b.close()
print("S03_POINTER_DIAGNOSTIC_COMPLETED; no changes to third-party websites")
