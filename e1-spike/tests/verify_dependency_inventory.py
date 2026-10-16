#!/usr/bin/env python3
"""R09 exact Cargo.lock/metadata dependency inventory (not a vulnerability clearance).
Runs against pinned lockfile with fetched registries; no credentials required.
"""
import hashlib, json, subprocess, tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
MANIFEST=ROOT/"e1-spike/Cargo.toml"
LOCK=ROOT/"e1-spike/Cargo.lock"
OUT=Path("/tmp/e1-security-depth")
OUT.mkdir(parents=True,exist_ok=True)
before=hashlib.sha256(LOCK.read_bytes()).hexdigest()
p=subprocess.run(["cargo","metadata","--format-version","1","--locked","--offline","--manifest-path",str(MANIFEST)],check=True,capture_output=True,text=True)
metadata=json.loads(p.stdout)
lock=tomllib.loads(LOCK.read_text())
locked={(p["name"],p["version"]):p for p in lock["package"]}
assert len(locked)==len(lock["package"]), "Duplicate name/version in Cargo.lock"
items=[]
for pkg in metadata["packages"]:
    key=(pkg["name"],pkg["version"])
    assert key in locked,("Metadata drift",key)
    entry=locked[key]
    items.append({
      "name":pkg["name"],"version":pkg["version"],
      "source":pkg.get("source"),"checksum_sha256":entry.get("checksum"),
      "license_declared_by_package":pkg.get("license"),
      "license_file":pkg.get("license_file"),
      "repository":pkg.get("repository"),
      "rust_version":pkg.get("rust_version")
    })
assert len(items)==len(locked),(len(items),len(locked))
after=hashlib.sha256(LOCK.read_bytes()).hexdigest()
assert before==after,"Cargo.lock changed during inventory"
missing=[f"{p['name']}@{p['version']}" for p in items if not p["license_declared_by_package"]]
report={
 "scope":"All packages in exact locked native+web cargo dependency graph",
 "manifest":"e1-spike/Cargo.toml",
 "cargo_lock_sha256":after,
 "package_count":len(items),
 "packages":sorted(items,key=lambda x:(x["name"],x["version"])),
 "missing_license_metadata":missing,
 "critical_limitation":"Package-declared license is NOT independent license audit; vulnerability DB/OSV/RustSec lookup is NOT performed here.",
 "review_verdict":"DEPENDENCY_INVENTORY_ONLY_NOT_R09_SIGNOFF"
}
(OUT/"r09-locked-dependency-inventory.json").write_text(json.dumps(report,indent=2)+"\n")
print("E1_R09_LOCKED_DEPENDENCY_INVENTORY_CREATED",len(items),"MISSING_LICENSE_FIELDS",len(missing),"LOCK_SHA256",after)
