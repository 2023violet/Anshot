#!/usr/bin/env python3
"""S03 scripted synthetic-image benchmark in *real* Chrome competitor UIs.

Uses manifest oracle rectangles, NOT human discovery. Each outcome is separately
labeled and output bytes independently inspected. Never treats import as completion.
"""
import json, hashlib, time, platform
from pathlib import Path
from PIL import Image, ImageChops
from playwright.sync_api import sync_playwright

base=Path("docs/research/fixtures")
manifest={v["id"]:v for v in json.loads((base/"manifest.json").read_text(encoding="utf8"))["fixtures"]}
out=Path("/tmp/shareready-s03-neutral-reset-runs")
out.mkdir(parents=True,exist_ok=True)
results=[]
targets=[
    ("Redacted","https://redacted.app/"),
    ("Redaction Tools","https://redactiontools.com/redact-screenshot"),
]
tasks=["T01_CHAT","T02_ORDER"]

def inspect_output(entry,item,dest):
    with Image.open(base/item["file"]) as source, Image.open(dest) as result:
        source=source.convert("RGB")
        result.load()
        entry.update({"export_format":result.format,"export_dimensions":result.size,
                      "export_sha256":hashlib.sha256(dest.read_bytes()).hexdigest(),
                      "export_bytes":dest.stat().st_size,
                      "original_dimensions":[item["width"],item["height"]],
                      "export_exif_present":bool(result.info.get("exif"))})
        if result.size!=source.size:
            entry["comparison"]="OUTPUT_DIMENSION_MISMATCH"
            return
        output=result.convert("RGB")
        redactions=[]
        for region in item["sensitive_regions"]:
            x1,y1,x2,y2=region["rect_px"]
            before=source.crop((x1,y1,x2,y2))
            after=output.crop((x1,y1,x2,y2))
            diff=ImageChops.difference(before,after)
            redactions.append({"id":region["id"],"changed_some_pixels":diff.getbbox() is not None,
                               "center_output_rgb":output.getpixel(((x1+x2)//2,(y1+y2)//2))})
        keeps=[]
        for region in item["must_preserve_regions"]:
            x1,y1,x2,y2=region["rect_px"]
            diff=ImageChops.difference(source.crop((x1,y1,x2,y2)),output.crop((x1,y1,x2,y2)))
            keeps.append({"id":region["id"],"unchanged":diff.getbbox() is None})
        entry.update({"region_analysis":redactions,"preservation_analysis":keeps,
                      "comparison":"PIXEL_DIFF_DIAGNOSTICS_ONLY_NOT_FULL_SECURITY_PROOF"})

with sync_playwright() as pw:
    browser=pw.chromium.launch(channel="chrome",headless=True,args=["--no-sandbox","--disable-dev-shm-usage"])
    ctx=browser.new_context(viewport={"width":1280,"height":900},locale="en-US",accept_downloads=True)
    for name,url in targets:
        for task_id in tasks:
            item=manifest[task_id]
            e={"tool":name,"task_id":task_id,"task_label":"oracle-assisted scripted browser operation",
               "environment":{"os":platform.platform(),"browser":"Chrome","version":browser.version,
                              "headless":True,"viewport":"1280x900","locale":"en-US"},
               "redaction_ai_setting":"off / no optional AI enabled","protocol_revision":"verified_neutral_click_between_manual_masks","status":"NOT_EXECUTED"}
            page=ctx.new_page()
            posts=[]
            errors=[]
            page.on("pageerror",lambda er: errors.append(str(er)[:300]))
            page.on("request",lambda req:posts.append({"host":req.url.split("/")[2],"method":req.method,"resource":req.resource_type}) if req.method=="POST" and "://" in req.url else None)
            try:
                page.goto(url,wait_until="domcontentloaded",timeout=45000)
                page.wait_for_timeout(1300)
                start=time.perf_counter()
                page.locator('input[type=file]').first.set_input_files(str((base/item["file"]).resolve()),timeout=15000)
                page.wait_for_timeout(5600)
                canvas=page.locator("canvas").first
                assert canvas.count()==1,"No actual canvas after import"
                canvas.scroll_into_view_if_needed(timeout=8000)
                dim=canvas.evaluate("(e)=>({w:e.width,h:e.height,clientW:e.getBoundingClientRect().width,clientH:e.getBoundingClientRect().height})")
                bounds=canvas.bounding_box()
                assert bounds and bounds["width"]>250 and bounds["height"]>150,"Canvas not visible"
                e["canvas_geometry"]={"intrinsic":dim,"bbox":bounds}
                e["input_file_was_provided"]=True
                if name=="Redacted":
                    page.get_by_role("button",name="Drag to draw box redactions").click(timeout=8000)
                    regions=item["sensitive_regions"]
                else:
                    regions=[r for r in item["sensitive_regions"] if r["id"] not in ("demo_phone","phone")]
                    e["default_auto_mask_count"]=1 if "1 hidden" in page.locator("body").inner_text(timeout=10000) else None
                e["manual_drag_targets"]=[r["id"] for r in regions]
                e["neutral_reset_clicks"]=0
                e["mask_addition_trace"]=[]
                for drag_index,area in enumerate(regions):
                    if name=="Redaction Tools" and drag_index>0:
                        page.mouse.move(1200,410)
                        page.mouse.click(1200,410)
                        page.wait_for_timeout(200)
                        e["neutral_reset_clicks"]+=1
                    bounds=canvas.bounding_box()
                    dim=canvas.evaluate("(el)=>({w:el.width,h:el.height,clientW:el.getBoundingClientRect().width,clientH:el.getBoundingClientRect().height})")
                    assert bounds,"Canvas not present before manual masking"
                    x1,y1,x2,y2=area["rect_px"]
                    # Only if canvas source-coordinates are correct: stop if geometry is implausible.
                    if not(0.7<=dim["w"]/item["width"]<=1.3 and 0.7<=dim["h"]/item["height"]<=1.3):
                        raise RuntimeError("Canvas intrinsic dimensions differ from input, cannot safely map coordinates")
                    scale_x=bounds["width"]/item["width"]
                    scale_y=bounds["height"]/item["height"]
                    sx=bounds["x"]+(x1+3)*scale_x; sy=bounds["y"]+(y1+3)*scale_y
                    ex=bounds["x"]+(x2-3)*scale_x; ey=bounds["y"]+(y2-3)*scale_y
                    page.mouse.move(sx,sy)
                    page.mouse.down()
                    page.mouse.move(ex,ey,steps=12)
                    page.mouse.up()
                    page.wait_for_timeout(440)
                    e["mask_addition_trace"].append({
                        "target":area["id"],
                        "manual_overlays":page.locator('button[title*="Drawn by you"]').count(),
                        "after_label": [v.strip() for v in page.locator("body").inner_text().splitlines() if "hidden" in v.lower()][:3]
                    })
                (out/(name.replace(" ","_").lower()+"_"+task_id+"_editor.jpg")).write_bytes(page.screenshot(type="jpeg",quality=56))
                button=page.get_by_role("button",name="Download PNG" if name=="Redaction Tools" else "Download",exact=True)
                assert button.count()==1,"Download button not found"
                with page.expect_download(timeout=20000) as ev:
                    button.click(timeout=10000)
                download=ev.value
                ext=".png" if download.suggested_filename.lower().endswith(".png") else ".jpg"
                saved=out/(name.replace(" ","_").lower()+"_"+task_id+"_export"+ext)
                download.save_as(saved)
                e["suggested_filename"]=download.suggested_filename
                e["elapsed_scripted_s"]=round(time.perf_counter()-start,3)
                inspect_output(e,item,saved)
                e["status"]="REAL_UI_DOWNLOAD_COMPLETE_NEEDS_OUTPUT_REVIEW"
            except Exception as exc:
                e.update({"status":"BLOCKED_OR_INCOMPLETE","error":str(exc)[:900]})
                try:
                    (out/(name.replace(" ","_").lower()+"_"+task_id+"_failure.jpg")).write_bytes(page.screenshot(type="jpeg",quality=55))
                except Exception:pass
            finally:
                e["post_hosts_observed"]=sorted({r["host"] for r in posts})
                e["page_errors"]=errors[:8]
                print("S03_SCRIPTED_COMPETITOR_RESULT",json.dumps(e,ensure_ascii=False))
                results.append(e)
                page.close()
    ctx.close()
    browser.close()
(out/"results.json").write_text(json.dumps({"schema":1,"date":"2026-10-14","comparison_class":"oracle-assisted scripted browser flow with verified neutral-click reset",
  "not_human_ux_benchmark":True,"a1_user_need_validation":"NOT_VERIFIED","results":results},indent=2,ensure_ascii=False)+"\n",encoding="utf8")
print("S03_SCRIPTED_BROWSER_RUNS_COMPLETE; RESULTS_SEPARATELY_CLASSIFIED")
