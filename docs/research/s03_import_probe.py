#!/usr/bin/env python3
"""Research-only scripted input probe on real online tools, not benchmark task completion."""
import base64
import json
import platform
import time
from pathlib import Path
from playwright.sync_api import sync_playwright

OUT=Path("/tmp/share-ready-s03-import")
OUT.mkdir(parents=True,exist_ok=True)
root=Path("docs/research/fixtures")
targets=[
    ("REDACTED","https://redacted.app/"),
    ("REDACTION_TOOLS","https://redactiontools.com/redact-screenshot")
]
fixtures=["T01_CHAT.png","T02_ORDER.png"]
res=[]
with sync_playwright() as pw:
    browser=pw.chromium.launch(headless=True,channel="chrome",args=["--disable-dev-shm-usage","--no-sandbox"])
    ctx=browser.new_context(viewport={"width":1280,"height":900},locale="en-US",accept_downloads=True)
    for name,url in targets:
        for fixture in fixtures:
            page=ctx.new_page()
            errors=[]
            network=[]
            page.on("pageerror",lambda err:errors.append(str(err)[:250]))
            page.on("request",lambda req:network.append({"method":req.method,"resource":req.resource_type,"url":req.url[:160]}))
            result={"tool":name,"fixture":fixture,"os":platform.platform(),"browser":"Chrome","version":browser.version,
                "viewport":"1280x900","headless":True,"status":"NOT_VERIFIED"}
            try:
                page.goto(url,wait_until="domcontentloaded",timeout=45000)
                page.wait_for_timeout(2000)
                file_input=page.locator("input[type=file]").first
                assert file_input.count()==1,"No file input"
                start=time.perf_counter()
                file_input.set_input_files(str((root/fixture).resolve()),timeout=15000)
                page.wait_for_timeout(8500)
                elapsed=round(time.perf_counter()-start,3)
                text=page.locator("body").inner_text(timeout=15000)
                result.update({"status":"FILE_PROVIDED_UI_INSPECTION_ONLY","seconds_since_file_selection_diagnostic":elapsed,
                    "url_after_input":page.url,"title":page.title(),"visible_text_excerpt":text[:4500],
                    "buttons":page.get_by_role("button").all_text_contents()[:70],
                    "input_file_count":page.locator("input[type=file]").count(),
                    "canvas_count":page.locator("canvas").count(),"svg_count":page.locator("svg").count(),
                    "image_count":page.locator("img").count(),
                    "button_attrs":page.evaluate("""() => Array.from(document.querySelectorAll('button')).slice(0,35).map(e => ({text:(e.innerText||e.ariaLabel||'').trim().slice(0,100),title:e.title,disabled:e.disabled}))""")})
                snap=page.screenshot(type="jpeg",quality=44,full_page=False)
                (OUT/(name.lower()+"_"+fixture.replace(".png","")+".jpg")).write_bytes(snap)
                if fixture=="T01_CHAT.png":
                    marker="S03_"+name+"_IMPORTED_T01_JPEG_BASE64_"
                    payload=base64.b64encode(snap).decode("ascii")
                    print(marker+"BEGIN")
                    for n in range(0,len(payload),1000):print(payload[n:n+1000])
                    print(marker+"END")
            except Exception as e:
                result.update({"status":"BLOCKED_BY_ENV_OR_SELECTOR","error":str(e)[:700]})
            finally:
                result["page_errors"]=errors[:8]
                result["requests_total"]=len(network)
                result["post_requests"]=[x for x in network if x["method"]=="POST"][:10]
                print("S03_IMPORT_UI_RESULT",json.dumps(result,ensure_ascii=False))
                res.append(result)
                page.close()
    ctx.close()
    browser.close()
(OUT/"import_probe_results.json").write_text(json.dumps({"schema":1,"type":"instrumented automation interface discovery",
    "not_a_task_benchmark":True,"completed_masking_export_runs":0,"entries":res},ensure_ascii=False,indent=2)+"\n",encoding="utf8")
print("S03_IMPORT_PROBE_COMPLETE; NO_MASK_EXPORT_TASK_SCORE")
