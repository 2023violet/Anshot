#!/usr/bin/env python3
"""R09 fresh, machine-readable OSV advisory lookup for every Cargo.lock package.
Sends only public crate names and versions to api.osv.dev; no image/user data.
Availability errors FAIL CLOSED. Findings must be independently triaged.
"""
import json, ssl, sys, urllib.error, urllib.request
from pathlib import Path
OUT=Path("/tmp/e1-security-depth")
inventory=json.loads((OUT/"r09-locked-dependency-inventory.json").read_text())
packages=[p for p in inventory["packages"] if p["source"] and "crates.io" in p["source"]]
queries=[{"package":{"ecosystem":"crates.io","name":p["name"]},"version":p["version"]} for p in packages]
body=json.dumps({"queries":queries}).encode()
request=urllib.request.Request("https://api.osv.dev/v1/querybatch",data=body,method="POST",
    headers={"Content-Type":"application/json","User-Agent":"ShareReady-security-review/1.0"})
try:
    with urllib.request.urlopen(request,context=ssl.create_default_context(),timeout=30) as response:
        assert response.status==200,response.status
        data=json.load(response)
except (urllib.error.URLError,TimeoutError,ValueError) as exc:
    result={"state":"LOOKUP_FAILED","error":str(exc),"queries":len(queries),
        "assurance":"NONE; NEVER TREAT NETWORK FAILURE AS CLEAN DEPENDENCIES"}
    (OUT/"r09-osv-lookup.json").write_text(json.dumps(result,indent=2)+"\n")
    print("E1_R09_OSV_LOOKUP_FAILED_CLOSED",str(exc),flush=True)
    sys.exit(3)
results=data.get("results",[])
assert len(results)==len(queries),(len(results),len(queries))
findings=[]
for pkg,entry in zip(packages,results):
    for advisory in entry.get("vulns",[]):
        findings.append({"package":pkg["name"],"version":pkg["version"],
                         "id":advisory.get("id"),"aliases":advisory.get("aliases",[]),
                         "modified":advisory.get("modified")})
result={"state":"FINDINGS_REVIEW_REQUIRED" if findings else "NO_OSV_MATCHES_IN_THIS_SNAPSHOT",
        "ecosystem":"crates.io",
        "queried":len(queries),
        "advisory_matches":findings,
        "limits":"OSV index snapshot only; not a proof of no vulnerabilities, license compatibility or future advisories; wasm-bindgen-cli build dependencies are NOT covered."}
(OUT/"r09-osv-lookup.json").write_text(json.dumps(result,indent=2)+"\n")
print("E1_R09_OSV_RESULTS",json.dumps(result),flush=True)
if findings:sys.exit(2)
