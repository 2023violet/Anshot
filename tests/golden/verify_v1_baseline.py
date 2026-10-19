#!/usr/bin/env python3
"""SR-V2-00: actual Rust CLI output versus an independent Pillow full-frame pixel oracle."""
import argparse,hashlib,io,json,os,struct,subprocess,tempfile,zlib
from pathlib import Path
from PIL import Image,ImageOps
ROOT=Path(__file__).resolve().parents[2]
FIX=ROOT/"docs/research/fixtures"
C=ROOT/"tests/golden/v1_3_baseline.json"
SIG=b"\x89PNG\r\n\x1a\n"
COVER=(17,24,39,255)
def hash_(b):return hashlib.sha256(b).hexdigest()
def png_chunks(raw,n):
    assert raw.startswith(SIG),(n,"not PNG")
    pos=8;seq=[]
    while pos<len(raw):
        assert pos+12<=len(raw),(n,"truncated chunk header")
        size=struct.unpack(">I",raw[pos:pos+4])[0]
        kind=raw[pos+4:pos+8]
        end=pos+12+size
        assert end<=len(raw),(n,"truncated PNG chunk")
        body=raw[pos+8:pos+8+size]
        assert struct.unpack(">I",raw[end-4:end])[0]==zlib.crc32(kind+body),(n,"CRC")
        assert kind in (b"IHDR",b"IDAT",b"IEND"),(n,"unexpected metadata/chunk",kind)
        seq.append(kind.decode("ascii"));pos=end
        if kind==b"IEND":
            assert size==0 and pos==len(raw),(n,"trailing bytes")
            break
    assert seq[0]=="IHDR" and seq[-1]=="IEND"
    assert seq.count("IHDR")==seq.count("IEND")==1 and "IDAT" in seq
    return seq
def main(args):
    contract=json.loads(C.read_text(encoding="utf-8"))
    manifest=json.loads((FIX/"manifest.json").read_text(encoding="utf-8"))
    reference={e["id"]:e for e in manifest["fixtures"]}
    assert len(reference)==len(contract["cases"])==5
    assert contract["provenance"]["trusted_cover_rgba"]==list(COVER)
    if not args.capture:
        assert contract["status"]=="FROZEN"
        assert all(e["expected_pixel_sha256"] for e in contract["cases"])
    suffix=".exe" if os.name=="nt" else ""
    exe=ROOT/("target/release/shareready-e1"+suffix)
    assert exe.is_file(),"cargo build --release --locked --manifest-path e1-spike/Cargo.toml first"
    results=[]
    with tempfile.TemporaryDirectory(prefix="shareready-v2-golden-") as out:
        for e in contract["cases"]:
            n=e["id"];m=reference[n];path=FIX/e["input"]
            src=path.read_bytes()
            assert m["file"]==e["input"] and hash_(src)==e["sha256"]==m["sha256"]
            assert e["dimensions"]==[m["width"],m["height"]]
            assert e["masks"]==[r["rect_px"] for r in m["sensitive_regions"]]
            pillow_normalized=ImageOps.exif_transpose(Image.open(io.BytesIO(src))).convert("RGBA")
            assert list(pillow_normalized.size)==e["dimensions"]
            oracle_source="PILLOW_FOR_LOSSLESS_SOURCE"
            norm=pillow_normalized
            if e["input"].lower().endswith((".jpg",".jpeg")):
                # JPEG decoders can differ by rounding and IDCT/color conversion.
                # Freeze V1 Rust's EXIF-normalized source pixels; Pillow still independently
                # composes mask truth and decodes/compares every output pixel.
                preview_bin=ROOT/("target/release/examples/v1_baseline_preview"+suffix)
                assert preview_bin.is_file(),"Build V1 Rust preview example first"
                preview=Path(out)/(n+"-normalized.png")
                p0=subprocess.run([str(preview_bin),str(path),str(preview)],capture_output=True,text=True,timeout=120)
                assert p0.returncode==0,(n,"preview failed",p0.stderr[-350:])
                png_chunks(preview.read_bytes(),n+"-normalized")
                norm=Image.open(io.BytesIO(preview.read_bytes())).convert("RGBA")
                assert list(norm.size)==e["dimensions"]
                oracle_source="RUST_V1_NORMALIZED_JPEG_PREVIEW_PLUS_INDEPENDENT_PILLOW_COMPOSITOR"
                print("SR_V2_00_JPEG_DECODER_DIFF",n,"PILLOW_SOURCE_RGBA_SHA256="+hash_(pillow_normalized.tobytes()),"RUST_PREVIEW_RGBA_SHA256="+hash_(norm.tobytes()),"EQUAL="+str(pillow_normalized.tobytes()==norm.tobytes()),flush=True)
            oracle=norm.copy()
            for l,t,r,b in e["masks"]:
                assert 0<=l<r<=oracle.width and 0<=t<b<=oracle.height
                oracle.paste(COVER,(l,t,r,b))
            expected_hash=hash_(oracle.tobytes())
            if e["expected_pixel_sha256"] is not None:
                assert expected_hash==e["expected_pixel_sha256"],(n,"frozen hash drift")
            dst=Path(out)/(n+".png")
            args_=[str(exe),str(path),str(dst)]+[",".join(map(str,r)) for r in e["masks"]]
            p=subprocess.run(args_,capture_output=True,text=True,timeout=120)
            assert p.returncode==0,(n,p.returncode,p.stderr[-350:])
            png=dst.read_bytes();chunks=png_chunks(png,n)
            decoded=Image.open(io.BytesIO(png)).convert("RGBA")
            assert decoded.size==oracle.size
            assert decoded.tobytes()==oracle.tobytes(),(n,"full image pixels mismatch")
            assert hash_(path.read_bytes())==e["sha256"],(n,"source modified")
            rec=dict(id=n,rgba_sha256=expected_hash,png_sha256=hash_(png),png_bytes=len(png),chunks=chunks,oracle_source=oracle_source)
            results.append(rec)
            print("SR_V2_00_CASE",n,"RGBA_SHA256="+expected_hash,"PNG_SHA256="+rec["png_sha256"],"FULL_IMAGE_PASS",flush=True)
    status="CAPTURE_ONLY_NOT_FROZEN" if args.capture else "FROZEN_GOLDEN_VERIFIED"
    args.report.parent.mkdir(parents=True,exist_ok=True)
    args.report.write_text(json.dumps(dict(status=status,git_sha=os.getenv("GITHUB_SHA","LOCAL"),cases=results),indent=2)+"\n")
    print("SR_V2_00_"+status,flush=True)
if __name__=="__main__":
    ap=argparse.ArgumentParser()
    ap.add_argument("--capture",action="store_true")
    ap.add_argument("--report",type=Path,default=Path("/tmp/shareready-v2-00/golden-report.json"))
    main(ap.parse_args())
