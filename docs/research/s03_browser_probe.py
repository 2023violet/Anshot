#!/usr/bin/env python3
"""S03 research-only real Chromium interface probe; never score page load as completed redaction."""
import base64
import json
import os
import platform
from pathlib import Path
from playwright.sync_api import sync_playwright

OUT = Path("/tmp/share-ready-s03-probe")
OUT.mkdir(parents=True, exist_ok=True)
TARGETS = [
    ("REDACTED", "https://redacted.app/"),
    ("REDACTION_TOOLS", "https://redactiontools.com/redact-screenshot"),
]
with sync_playwright() as pw:
    browser = pw.chromium.launch(headless=True, channel="chrome",
        args=["--disable-dev-shm-usage","--no-sandbox"])
    ctx = browser.new_context(viewport={"width":1280,"height":900},locale="en-US",
        accept_downloads=True, permissions=[])
    results=[]
    for name, url in TARGETS:
        page=ctx.new_page()
        failures=[]
        errors=[]
        reqs=[]
        page.on("pageerror",lambda e: errors.append(str(e)[:400]))
        page.on("request",lambda r: reqs.append({"method":r.method,"host":r.url.split("/")[2] if "://" in r.url else "",
            "kind":r.resource_type}))
        entry={"tool":name,"url":url,"status":"UNKNOWN","environment":{
            "os":platform.platform(),"browser":"Chromium Chrome channel","version":browser.version,
            "viewport":"1280x900","locale":"en-US","headless":True}}
        try:
            response=page.goto(url,wait_until="domcontentloaded",timeout=45000)
            page.wait_for_timeout(3500)
            entry.update({
                "http_status":response.status if response else None,
                "title":page.title(),
                "landed_url":page.url,
                "visible_text_excerpt":page.locator("body").inner_text(timeout=10000)[:2500],
                "file_inputs":page.locator("input[type=file]").count(),
                "canvas_elements":page.locator("canvas").count(),
                "textarea_elements":page.locator("textarea").count(),
                "buttons":page.get_by_role("button").all_text_contents()[:35],
                "contenteditable_elements":page.locator("[contenteditable=true]").count(),
                "javascript_loaded":page.evaluate("() => document.readyState"),
                "api_clipboard":page.evaluate("() => !!(navigator.clipboard && navigator.clipboard.write)"),
            })
            entry["status"]="PAGE_INTERACTIVE_REVIEW_NEEDED" if entry["http_status"]==200 else "HTTP_UNEXPECTED"
            jpg=page.screenshot(type="jpeg",quality=45,full_page=False)
            (OUT/(name.lower()+".jpg")).write_bytes(jpg)
            print("S03_"+name+"_ACTUAL_BROWSER_SCREENSHOT_BASE64_BEGIN")
            enc=base64.b64encode(jpg).decode("ascii")
            for pos in range(0,len(enc),1000):
                print(enc[pos:pos+1000])
            print("S03_"+name+"_ACTUAL_BROWSER_SCREENSHOT_BASE64_END")
        except Exception as exc:
            entry.update({"status":"BLOCKED_BY_ENV","error":str(exc)[:1100]})
        finally:
            entry["page_errors"]=errors[:8]
            entry["network_requests_total"]=len(reqs)
            entry["network_methods"]={method:sum(1 for x in reqs if x["method"]==method) for method in sorted({x["method"] for x in reqs})}
            results.append(entry)
            print("S03_BROWSER_PROBE_RESULT",json.dumps({k:v for k,v in entry.items() if k not in ("visible_text_excerpt","buttons")},ensure_ascii=False))
            page.close()
    ctx.close()
    browser.close()
(OUT/"probe_results.json").write_text(json.dumps({"schema":1,"observed_not_measured":True,
    "tools":results,"interpretation":"Headless page availability is not a redaction task benchmark, security assessment, or human usability test."},indent=2,ensure_ascii=False)+"\n",encoding="utf8")
print("S03_PROBE_COMPLETED: BROWSER_NAVIGATION_TEST_ONLY; NO_HUMAN_SESSIONS; NO_EXPORT_TIMINGS")
