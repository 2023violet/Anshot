#!/usr/bin/env python3
"""Actual Chromium canvas interaction tests. All imagery is synthetic."""
import io
import json
import http.server
import threading
import time
from pathlib import Path
from PIL import Image
from playwright.sync_api import sync_playwright

ROOT=Path(__file__).resolve().parents[2]
OUT=Path("/tmp/shareready-canvas-v1302")
OUT.mkdir(parents=True,exist_ok=True)
image=Image.new("RGB",(640,400),(231,241,250))
buf=io.BytesIO();image.save(buf,"PNG")
fixture=OUT/"synthetic-pointer-coordinate-grid.png"
fixture.write_bytes(buf.getvalue())

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self,*a,**kw):super().__init__(*a,directory=str(ROOT),**kw)
    def log_message(self,*a):pass

server=http.server.ThreadingHTTPServer(("127.0.0.1",0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
origin=f"http://127.0.0.1:{server.server_port}"
records=[]

def wait(page,expression,timeout=30000):
    # CSP forbids unsafe evaluation; call a trusted JavaScript function.
    end=time.monotonic()+timeout/1000
    while time.monotonic()<end:
        if page.evaluate(f"() => Boolean({expression})"):return
        page.wait_for_timeout(100)
    raise AssertionError("Timed out: "+expression)

def get_rect(page):
    r=page.evaluate("window.e2Debug.regions[0]")
    return [r["x0"],r["y0"],r["x1"],r["y1"]]

def set_rect(page,rect):
    # Click inspector region to reselect after Undo; changes must be genuinely
    # handled by the application and enter the editing history.
    page.locator("#regionList .region").first.click()
    for name,value in zip(("precX","precY","precW","precH"),
                          (rect[0],rect[1],rect[2]-rect[0],rect[3]-rect[1])):
        field=page.locator("#"+name)
        field.fill(str(value));field.press("Tab")
    assert get_rect(page)==rect,(get_rect(page),rect)

def center(box):
    return box["x"]+box["width"]/2,box["y"]+box["height"]/2

def move_mouse(page,start,end):
    page.mouse.move(*start)
    page.mouse.down()
    page.mouse.move(*end,steps=5)
    page.mouse.up()

with sync_playwright() as pw:
    browser=pw.chromium.launch(channel="chrome",headless=True,args=["--no-sandbox"])
    try:
        for dpr in (1,2):
            ctx=browser.new_context(
                device_scale_factor=dpr,viewport={"width":1440,"height":900},
                accept_downloads=True)
            page=ctx.new_page()
            errors=[]
            page.on("pageerror",lambda e:errors.append(str(e)))
            page.goto(origin+"/web-app/index.html")
            wait(page,"window.e2Debug?.ready === true")
            page.locator("#pick").set_input_files(str(fixture))
            wait(page,"window.e2Debug?.stage==='edit'")
            page.locator("#createMask").click()
            assert len(page.locator(".mask").all())==1
            # Real eight-direction handles have distinct direction markers.
            directions=("nw","n","ne","e","se","s","sw","w")
            page.locator("#regionList .region").click()
            assert sorted(page.locator('.handle').evaluate_all(
                "items => items.map(i=>i.dataset.dir)"))==sorted(directions)
            base=[140,100,320,230]
            for direction in directions:
                set_rect(page,base)
                original_revision=page.evaluate("window.e2Debug.revision")
                handle=page.locator(".handle-"+direction)
                assert handle.count()==1
                sx,sy=center(handle.bounding_box())
                frame=page.locator("#frame").bounding_box()
                scale_x=frame["width"]/640
                scale_y=frame["height"]/400
                dx=(-18 if "w" in direction else 18 if "e" in direction else 0)
                dy=(-12 if "n" in direction else 12 if "s" in direction else 0)
                move_mouse(page,(sx,sy),(sx+dx*scale_x,sy+dy*scale_y))
                expected=[
                    140+dx if "w" in direction else 140,
                    100+dy if "n" in direction else 100,
                    320+dx if "e" in direction else 320,
                    230+dy if "s" in direction else 230
                ]
                assert get_rect(page)==expected,(dpr,direction,get_rect(page),expected)
                assert page.evaluate("window.e2Debug.revision")==original_revision+1
                assert page.locator(".handle").count()==8
                records.append({"dpr":dpr,"case":"direction "+direction,"rect":expected})
            # Moving a selected rectangle creates exactly one Undo action.
            set_rect(page,base)
            before=page.evaluate("window.e2Debug.revision")
            mask=page.locator(".mask").bounding_box()
            sx=mask["x"]+mask["width"]*.25
            sy=mask["y"]+mask["height"]*.3
            frame=page.locator("#frame").bounding_box()
            move_mouse(page,(sx,sy),(sx+17*frame["width"]/640,sy+9*frame["height"]/400))
            assert get_rect(page)==[157,109,337,239],get_rect(page)
            assert page.evaluate("window.e2Debug.revision")==before+1
            page.locator("#stageLabel").click()
            page.keyboard.press("Control+z")
            assert get_rect(page)==base,get_rect(page)
            page.keyboard.press("Control+Shift+z")
            assert get_rect(page)==[157,109,337,239]
            records.append({"dpr":dpr,"case":"move + undo/redo"})

            # Test zoom + HTML page scale/DPR correctness in image pixel space.
            page.evaluate("document.body.style.zoom='1.25'")
            page.locator("#maskTool").click()
            page.locator("#zoomFit").click()
            frame=page.locator("#frame").bounding_box()
            scaleX=frame["width"]/640
            scaleY=frame["height"]/400
            # Use a second, non-overlapping rectangle near the top-left.
            start=(frame["x"]+25*scaleX,frame["y"]+20*scaleY)
            end=(frame["x"]+110*scaleX,frame["y"]+76*scaleY)
            move_mouse(page,start,end)
            assert len(page.evaluate("window.e2Debug.regions"))==2
            second=page.evaluate("window.e2Debug.regions[1]")
            assert [second[k] for k in ("x0","y0","x1","y1")]==[25,20,110,76],second
            # The inspector will not change existing first mask coordinates.
            assert get_rect(page)==[157,109,337,239]
            # A bounds clamp must never create a zero-area mask.
            page.locator('#regionList .region').first.click()
            set_rect(page,base)
            h=page.locator(".handle-w").bounding_box()
            sx,sy=center(h)
            right=page.locator("#frame").bounding_box()
            move_mouse(page,(sx,sy),(right["x"]+(319)*right["width"]/640,sy))
            assert get_rect(page)==[319,100,320,230],get_rect(page)
            records.append({"dpr":dpr,"case":"min 1px clamp + 125% page zoom"})

            # Pan at 2x through blank checkerboard and Space over image;
            # never modifies region geometry or output revision.
            page.evaluate("document.body.style.zoom='1'")
            page.locator("#zoomActual").click()
            for _ in range(4):page.locator("#zoomIn").click()
            before_regions=page.evaluate("window.e2Debug.regions")
            vp=page.locator("#viewport")
            vp.evaluate("(e)=>{e.scrollLeft=180;e.scrollTop=130}")
            scroll1=vp.evaluate("(e)=>[e.scrollLeft,e.scrollTop]")
            page.locator("#panTool").click()
            box=vp.bounding_box()
            start=(box["x"]+20,box["y"]+60)
            move_mouse(page,start,(start[0]+55,start[1]+40))
            scroll2=vp.evaluate("(e)=>[e.scrollLeft,e.scrollTop]")
            assert scroll2[0]<scroll1[0] and scroll2[1]<scroll1[1],(scroll1,scroll2)
            assert page.evaluate("window.e2Debug.regions")==before_regions
            records.append({"dpr":dpr,"case":"pan off-canvas with H tool"})
            page.locator("#selectTool").click()
            page.locator("#stageLabel").click()
            vp.evaluate("(e)=>{e.scrollLeft=210;e.scrollTop=150}")
            scroll1=vp.evaluate("(e)=>[e.scrollLeft,e.scrollTop]")
            mask=page.locator(".mask").first.bounding_box()
            sx,sy=mask["x"]+mask["width"]*.3,mask["y"]+mask["height"]*.3
            page.keyboard.down("Space")
            move_mouse(page,(sx,sy),(sx+35,sy+24))
            page.keyboard.up("Space")
            scroll2=vp.evaluate("(e)=>[e.scrollLeft,e.scrollTop]")
            assert scroll2[0]<scroll1[0] and scroll2[1]<scroll1[1],(scroll1,scroll2)
            assert page.evaluate("window.e2Debug.regions")==before_regions
            records.append({"dpr":dpr,"case":"Space to pan without moving masks"})

            # Export and verify actual Rust pixel buffer instead of a CSS layer.
            page.locator("#check").click()
            wait(page,"window.e2Debug?.stage==='review' && window.e2Debug.outputBytes>0")
            with page.expect_download() as dl:page.locator("#download").click()
            saved=OUT/f"canvas-dpr-{dpr}-real-redaction.png"
            dl.value.save_as(saved)
            with Image.open(saved) as actual:
                assert actual.size==(640,400)
                rgba=actual.convert("RGBA")
                for r in before_regions:
                    x0=int(min(r["x0"],r["x1"]))
                    y0=int(min(r["y0"],r["y1"]))
                    x1=int(max(r["x0"],r["x1"]))
                    y1=int(max(r["y0"],r["y1"]))
                    for y in range(y0,y1):
                        for x in range(x0,x1):
                            assert rgba.getpixel((x,y))==(17,24,39,255)
                assert rgba.getpixel((2,2))==(231,241,250,255)
            assert not errors,errors
            records.append({"dpr":dpr,"case":"Rust PNG pixels match masks at DPR and page zoom"})
            ctx.close()
    finally:
        browser.close()
        server.shutdown()
report={"result":"PASS","browser":"real Google Chrome","tested_dpr":[1,2],
        "canvas_cases":records,"image_source":"local synthetic PNG only"}
(OUT/"canvas-results.json").write_text(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print("SHAREREADY_V1302_EIGHT_GRIPS_PAN_DPR_EXPORT_PASS",len(records))
