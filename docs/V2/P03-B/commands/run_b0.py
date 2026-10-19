"""Bound B0 observations. Invoke only after the displayed Gate B is approved."""
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    mode, context_path, expected_context = sys.argv[1:]
    assert mode in ("identity", "probe", "regression")
    assert sha(context_path) == expected_context, "context identity changed"
    ctx = json.loads(Path(context_path).read_text(encoding="utf-8"))
    root = Path(ctx["root"]).resolve()
    assert Path.cwd().resolve() == root, "run from the bound repository"
    assert sha(__file__) == ctx["runner_sha256"], "runner changed"
    head = subprocess.check_output([ctx["git"], "rev-parse", "HEAD"], cwd=root, text=True).strip()
    assert head == ctx["head"], "HEAD changed"
    branch = subprocess.check_output([ctx["git"], "branch", "--show-current"], cwd=root, text=True).strip()
    assert branch == "main", "main-only policy"
    subprocess.run([ctx["git"], "diff", "--exit-code", "HEAD", "--", ".", ":(exclude)docs/**"], cwd=root, check=True)
    dirty = subprocess.check_output([ctx["git"], "status", "--porcelain", "-uall"], cwd=root, text=True)
    assert all(line[3:].replace("\\", "/").startswith("docs/") for line in dirty.splitlines()), "unexpected changes outside docs"
    for name, expected in ctx["files"].items():
        assert sha(root / name) == expected, "bound source/fixture changed: " + name
    for name, item in ctx["tools"].items():
        assert sha(item["path"]) == item["sha256"], "tool changed: " + name
    assert sha(ctx["exe"]) == ctx["exe_sha256"], "EXE changed"
    output = (root / ctx["output"]).resolve()
    assert output.is_relative_to(root / "target" / "p03-b-evidence"), "output outside bound test directory"
    attempt = output / (dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S") + "-" + mode + "-" + uuid.uuid4().hex[:8])
    attempt.mkdir(parents=True, exist_ok=False)
    report = {"mode": mode, "utc": dt.datetime.now(dt.timezone.utc).isoformat(), "head": head,
              "context_sha256": expected_context, "exe_sha256": ctx["exe_sha256"],
              "dirty": dirty, "results": [], "human_acceptance": "NOT_RUN"}
    env = os.environ.copy()
    env["RUSTC"] = ctx["tools"]["rustc"]["path"]
    env["RUSTDOC"] = ctx["tools"]["rustdoc"]["path"]
    env["PATH"] = str(Path(env["RUSTC"]).parent) + os.pathsep + env.get("PATH", "")
    env["CARGO_TARGET_DIR"] = str(root / "target")
    env["WGPU_BACKEND"] = "dx12"

    def run(label, args, timeout, required=()):
        started = dt.datetime.now(dt.timezone.utc)
        out = attempt / (label + ".stdout.txt")
        err = attempt / (label + ".stderr.txt")
        result = {"label": label, "args": args, "started_utc": started.isoformat()}
        with out.open("xb") as stdout, err.open("xb") as stderr:
            try:
                child = subprocess.run(args, cwd=root, env=env, stdout=stdout, stderr=stderr,
                                       timeout=timeout, creationflags=subprocess.CREATE_NO_WINDOW)
                result["exit_code"] = child.returncode
            except subprocess.TimeoutExpired:
                result.update(exit_code=None, timed_out=True)
        text = out.read_text(encoding="utf-8", errors="replace")
        result["passed"] = result["exit_code"] == 0 and all(marker in text for marker in required)
        result["seconds"] = (dt.datetime.now(dt.timezone.utc) - started).total_seconds()
        result["files"] = {out.name: sha(out), err.name: sha(err)}
        if label == "wgpu-dx12":
            result["adapter_lines"] = [line for line in text.splitlines() if line.startswith("SR_V2_03_GPU_ADAPTER")]
            result["physical_gpu_acceptance"] = "NOT_VERIFIED"
            result["passed"] &= bool(result["adapter_lines"]) and "backend=Dx12" in text
        report["results"].append(result)
        (attempt / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        assert result["passed"], label + " failed; inspect " + str(attempt)

    if mode == "identity":
        run("cargo-version", [ctx["tools"]["cargo"]["path"], "--version"], 20)
        run("rustc-version", [ctx["tools"]["rustc"]["path"], "--version"], 20)
    elif mode == "probe":
        run("self-test", [ctx["exe"], "--self-test"], 60, ("SR_V2_03_NATIVE_POC_SELFTEST_PASS",))
        run("wgpu-dx12", [ctx["exe"], "--renderer", "wgpu", "--smoke-window"], 60,
            ("SR_V2_03_NATIVE_WINDOW_LIFECYCLE_PASS",))
    else:
        cargo = ctx["tools"]["cargo"]["path"]
        run("workspace-tests", [cargo, "test", "--workspace", "--locked"], 1200)
        run("workspace-check", [cargo, "check", "--workspace", "--all-targets", "--locked"], 1200)
    assert sha(ctx["exe"]) == ctx["exe_sha256"], "EXE changed during observation"
    for name, expected in ctx["files"].items():
        assert sha(root / name) == expected, "source/fixture changed during observation: " + name
    print("B0_MACHINE_OBSERVATION_COMPLETE " + str(attempt / "report.json"))
    print("GUI, physical GPU, pointer accuracy, dual-DPI and Narrator acceptance remain NOT_VERIFIED.")


if __name__ == "__main__":
    main()
