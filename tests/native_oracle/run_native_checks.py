"""Build a local native test artifact and bind file/window checks to its identity.

Writes only new evidence under ignored target/. Does not publish, install, alter
Windows settings, inject input, or assert physical/human acceptance.
"""
from pathlib import Path
import datetime
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys

from png_oracle import check_files
from PIL import __version__ as pillow_version
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    out = ROOT / "target/p03-c-evidence" / ("native-" + stamp)
    out.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ)
    cargo = Path(env["RUSTC"]).with_name("cargo.exe") if env.get("RUSTC") else Path(shutil.which("cargo") or "cargo")
    files = sorted({*ROOT.glob("poc/native-egui/src/*.rs"), *ROOT.glob("crates/shareready-core/src/**/*.rs"),
                    *ROOT.glob("crates/shareready-editor/src/**/*.rs"), *ROOT.glob("tests/native_oracle/*.py"),
                    *ROOT.glob("crates/shareready-platform-windows/src/**/*.rs"),
                    *ROOT.glob("crates/shareready-core/tests/**/*.rs"),
                    *ROOT.glob("poc/native-egui/input-bridge/src/**/*.rs"),
                    *ROOT.glob("poc/native-egui/examples/**/*.rs"),
                    *(p for p in ROOT.glob("poc/native-egui/assets/**/*") if p.is_file()),
                    *(p for p in ROOT.glob("tests/performance/fixtures/**/*") if p.is_file()),
                    *ROOT.glob("docs/research/fixtures/T0*"),
                    ROOT / "poc/native-egui/input-bridge/Cargo.toml",
                    ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "crates/shareready-platform-windows/Cargo.toml",
                    ROOT / "poc/native-egui/Cargo.toml", ROOT / "poc/native-egui/Cargo.lock",
                    ROOT / "crates/shareready-core/Cargo.toml", ROOT / "crates/shareready-editor/Cargo.toml",
                    ROOT / "docs/research/fixtures/T04_EDGES.png"})
    source = {p.relative_to(ROOT).as_posix(): digest(p) for p in files}
    build_id = hashlib.sha256(json.dumps(source, sort_keys=True).encode()).hexdigest()
    env.update(SR_BUILD_ID=build_id, WGPU_BACKEND="dx12")
    report = {"schema": 1, "utc": stamp, "build_id": build_id, "source_files": source,
              "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
              "window_scope": "automatic DX12 lifecycle only; no physical pointer/DPI, human usability or Narrator claim",
              "python":sys.version,"pillow":pillow_version,"checks": []}

    def run(label, args, timeout=300):
        result = subprocess.run([str(a) for a in args], cwd=ROOT, env=env, capture_output=True, timeout=timeout)
        (out / (label + ".stdout.txt")).write_bytes(result.stdout)
        (out / (label + ".stderr.txt")).write_bytes(result.stderr)
        report["checks"].append({"name": label, "exit_code": result.returncode,
                                 "stdout_sha256": hashlib.sha256(result.stdout).hexdigest(),
                                 "stderr_sha256": hashlib.sha256(result.stderr).hexdigest()})
        if result.returncode:
            raise RuntimeError(f"{label} failed ({result.returncode}); see {out.relative_to(ROOT)}")
        return result.stdout.decode("utf-8", errors="replace")

    try:
        run("oracle-tests",[sys.executable,"-m","unittest","discover","-s","tests/native_oracle","-v"])
        count=re.search(r"Ran (\d+) tests",(out / "oracle-tests.stderr.txt").read_text(encoding="utf-8"))
        if not count or int(count.group(1))<19:
            raise RuntimeError("oracle regression discovery ran fewer than 19 required tests")
        report["oracle_test_count"]=int(count.group(1))
        report["rustc"] = run("rustc-version", [env.get("RUSTC", "rustc"), "--version"] ).strip()
        run("build", [cargo, "build", "--manifest-path", "poc/native-egui/Cargo.toml", "--locked",
                      "--target-dir", ROOT / "poc/native-egui/target"])
        built = ROOT / "poc/native-egui/target/debug/shareready-native-egui-poc.exe"
        exe = out / built.name
        shutil.copyfile(built, exe)
        report["exe_sha256"] = digest(exe)
        output = run("selftest", [exe, "--self-test", "--self-test-output", out / "files"])
        if "SR_V2_03_NATIVE_POC_SELFTEST_PASS" not in output or "SR_V2_03_SAVE_NEW_HANDLE_BOUND_PASS" not in output:
            raise RuntimeError("selftest or handle-bound new-file save marker absent")
        report["handle_bound_selftest_save"]=True
        frozen=ROOT / "docs/research/fixtures/T04_EDGES.png"
        if (out / "files/source-T04.png").read_bytes()!=frozen.read_bytes():
            raise RuntimeError("selftest source differs from frozen T04")
        report["independent_file_oracle"] = check_files(frozen, out / "files/output.png", out / "files/regions.json")
        window = run("dx12-smoke", [exe, "--smoke-window", "--renderer", "wgpu", "--synthetic-trace", out / "window-trace"], timeout=60)
        if "SR_V2_03_NATIVE_WINDOW_LIFECYCLE_PASS" not in window or "SR_V2_03_GPU_ADAPTER backend=Dx12" not in window:
            raise RuntimeError("DX12 adapter or window lifecycle marker absent")
        frames = [json.loads(line) for line in (out / "window-trace/events.jsonl").read_text().splitlines()]
        if frames[0]["build"] != build_id or not any(f.get("type") == "frame" for f in frames):
            raise RuntimeError("window trace identity or frame absent")
        report["trace_frames"] = frames
        reviews=list((out / "window-trace").glob("review-*.png"))
        if len(reviews)!=1 or not any(f.get("reviewed") for f in frames):
            raise RuntimeError("actual window review trace absent")
        png=reviews[0]
        if (out / "window-trace/source-T04.png").read_bytes()!=frozen.read_bytes():
            raise RuntimeError("window trace source differs from frozen T04")
        report["window_review_oracle"] = check_files(frozen,png,png.with_suffix(".regions.json"))
        visual=out / "visual"
        output=run("visual",[exe,"--renderer","wgpu","--visual-evidence",visual],timeout=120)
        if "SR_VISUAL_EVIDENCE_PASS states=8" not in output or "SR_VISUAL_EVIDENCE_FAIL" in output:
            raise RuntimeError("synthetic GPU visual run did not complete")
        events=[json.loads(line) for line in (visual / "events.jsonl").read_text(encoding="utf-8").splitlines()]
        shots=[event for event in events if event.get("type")=="gpu_screenshot"]
        if events[0]["build"]!=build_id or [s["state"] for s in shots]!=["edit","multi","review","back-edit","error","discard","small","dark"]:
            raise RuntimeError("visual build identity or exact stage sequence mismatch")
        if [len(s["regions"]) for s in shots]!=[1,2,2,2,2,2,2,2] or not shots[2]["review_view"] or any(s["review_view"] for s in shots[3:]):
            raise RuntimeError("visual edit/review state mismatch")
        if any(s["revision"]!=shots[1]["revision"] for s in shots[2:]) or not all(s["reviewed"] for s in shots[2:]):
            raise RuntimeError("back/error/discard/resize changed the reviewed artifact")
        for shot in shots:
            path=visual / shot["png"]
            with Image.open(path) as image:
                image.load()
                if list(image.size)!=shot["size"] or image.convert("RGBA").getchannel("A").getextrema()!=(255,255):
                    raise RuntimeError("GPU image dimensions or opaque background invalid")
            shot["sha256"]=digest(path)
        if not (shots[6]["size"][0]<shots[0]["size"][0] and shots[6]["size"][1]<shots[0]["size"][1]):
            raise RuntimeError("actual small window readback did not resize")
        report["visual_screenshots"]=shots
        pngs=list(visual.glob("review-*.png"))
        if len(pngs)!=1:raise RuntimeError("visual review artifact count mismatch")
        report["visual_review_oracle"]=check_files(frozen,pngs[0],pngs[0].with_suffix(".regions.json"))
        if (visual / "source-T04.png").read_bytes()!=frozen.read_bytes():raise RuntimeError("visual source differs from frozen T04")
        if source != {p.relative_to(ROOT).as_posix(): digest(p) for p in files} or digest(exe) != report["exe_sha256"]:
            raise RuntimeError("source or artifact changed during verification")
        report["ok"] = True
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        report.update(ok=False, error=str(error))
    (out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"ok": report["ok"], "report": (out / "report.json").relative_to(ROOT).as_posix(),
                      "build_id": build_id, "exe_sha256": report.get("exe_sha256"), "error": report.get("error")}))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
