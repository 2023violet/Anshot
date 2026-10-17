#!/usr/bin/env python3
"""R04 bounded synthetic PNG adversarial CLI checks + observed native peak RSS.
Evidence characterization, NOT a proof that every hostile image is safe.
"""
import json, os, resource, struct, subprocess, tempfile, time, zlib
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
BIN=ROOT/"target/release/shareready-e1"
OUT=Path("/tmp/e1-security-depth")
OUT.mkdir(exist_ok=True,parents=True)
assert BIN.is_file(), BIN
SIG=b"\x89PNG\r\n\x1a\n"
def chunk(tag,payload):
    return struct.pack(">I",len(payload))+tag+payload+struct.pack(">I",zlib.crc32(tag+payload)&0xffffffff)

def png(width,height,valid_pixels=False):
    header=chunk(b"IHDR",struct.pack(">IIBBBBB",width,height,8,6,0,0,0))
    if not valid_pixels:
        return SIG+header+chunk(b"IDAT",zlib.compress(b""))+chunk(b"IEND",b"")
    compressor=zlib.compressobj(6)
    row=b"\0"+bytes((59,71,89,255))*width
    packed=bytearray()
    for _ in range(height):
        packed.extend(compressor.compress(row))
    packed.extend(compressor.flush())
    return SIG+header+chunk(b"IDAT",bytes(packed))+chunk(b"IEND",b"")

records=[]
def test(name,input_bytes,expect_success,expected_error=None):
    inp=OUT/(name+".png")
    dst=OUT/(name+"_output.png")
    inp.write_bytes(input_bytes)
    dst.unlink(missing_ok=True)
    tick=time.monotonic()
    command=[str(BIN),str(inp),str(dst),"0,0,1,1"]
    proc=subprocess.run(command,capture_output=True,text=True,timeout=90,check=False)
    elapsed_ms=round((time.monotonic()-tick)*1000,2)
    rss=resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    if expect_success:
        assert proc.returncode==0,(name,proc.stderr)
        assert dst.read_bytes().startswith(SIG),name
    else:
        assert proc.returncode==2,(name,proc.returncode,proc.stderr)
        assert not dst.exists(),(name,"failed export wrote a file")
        assert expected_error in proc.stderr,(name,proc.stderr)
    records.append({"scenario":name,"input_bytes":len(input_bytes),
        "process_exit":proc.returncode,"elapsed_ms":elapsed_ms,
        "max_child_rss_kib_observed_so_far":rss,
        "output_bytes":dst.stat().st_size if dst.exists() else 0,
        "error":proc.stderr.strip()[:300] if not expect_success else None})
    inp.unlink()
    if dst.exists():dst.unlink()
    print("E1_R04_RESOURCE_CASE",json.dumps(records[-1]))

test("pixels_36mp_header",png(6000,6000),False,"decoded pixel dimensions out of bounds")
# Input >25 MiB with otherwise valid PNG magic must be refused before decode.
huge=SIG+(b"\0"*(25*1024*1024+1-len(SIG)))
test("compressed_over_25mib",huge,False,"invalid or oversized compressed input")
del huge
# Deliberately truncated PNG with a plausible magic/header.
test("truncated_idat",png(800,600)[:45],False,"image")
# Valid 4.84MP highly compressed synthetic PNG exercises decode, opaque overwrite,
# new PNG encoder and readback; measured child RSS is an observation, not cap.
test("valid_compressed_4_84mp",png(2200,2200,True),True)
report={"scope":"single Linux runner, synthetic generated PNG only; not a wasm OOM guarantee",
        "compressed_size_limit_bytes":25*1024*1024,
        "decoded_pixel_limit":24_000_000,
        "cases":records,
        "limitations":["No Wasm peak RSS or OOM trap guarantee", "Only chosen hostile headers and compressibility pattern",
        "No full fuzz corpus or allocation-before-limit proof",
        "ru_maxrss is the cumulative maximum for completed child processes"]}
(OUT/"r04-native-resource-report.json").write_text(json.dumps(report,indent=2)+"\n")
print("E1_R04_ADVERSARIAL_NATIVE_CHARACTERIZATION_PASS")
