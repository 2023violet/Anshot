"""Read-only independent PNG-file oracle; never invokes a Rust encoder or modifies inputs.

JPEG cases must supply the frozen Rust-normalized PNG anchor, not a fresh Pillow JPEG decode.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import struct
import zlib

from PIL import Image

SIGNATURE = b"\x89PNG\r\n\x1a\n"
COVER = (17, 24, 39, 255)


def bounded_read(path, maximum):
    with Path(path).open("rb") as stream:
        raw = stream.read(maximum + 1)
    if len(raw) > maximum:
        raise ValueError("file exceeds oracle resource limit")
    return raw


def inspect_png(raw, strict_output):
    if not raw.startswith(SIGNATURE):
        raise ValueError("PNG or frozen normalized PNG required")
    pos, chunks, dimensions, data_closed, seen_data = 8, [], None, False, False
    compressed = []
    while pos < len(raw):
        if pos + 12 > len(raw):
            raise ValueError("truncated PNG chunk header")
        size = struct.unpack_from(">I", raw, pos)[0]
        kind = raw[pos + 4:pos + 8]
        end = pos + 12 + size
        if end > len(raw):
            raise ValueError("truncated PNG chunk")
        body = raw[pos + 8:end - 4]
        if struct.unpack_from(">I", raw, end - 4)[0] != zlib.crc32(kind + body):
            raise ValueError("PNG CRC mismatch")
        if strict_output and kind not in (b"IHDR", b"IDAT", b"IEND"):
            raise ValueError("output contains metadata or unexpected chunks")
        if kind == b"IHDR":
            if chunks or size != 13:
                raise ValueError("invalid or repeated IHDR")
            w, h, depth, color, compression, filtering, interlace = struct.unpack(">IIBBBBB", body)
            if not w or not h or w * h > 24_000_000:
                raise ValueError("PNG dimensions exceed resource contract")
            if strict_output and (depth, color, compression, filtering, interlace) != (8, 6, 0, 0, 0):
                raise ValueError("output must be non-interlaced RGBA8 PNG")
            dimensions = (w, h)
        elif dimensions is None:
            raise ValueError("IHDR must be first")
        if kind == b"IDAT" and data_closed:
            raise ValueError("non-contiguous IDAT")
        if kind == b"IDAT":
            seen_data = True
            if strict_output:
                compressed.append(body)
        elif seen_data:
            data_closed = True
        chunks.append(kind)
        pos = end
        if kind == b"IEND":
            if size or pos != len(raw):
                raise ValueError("invalid IEND or trailing payload")
            break
    if not chunks or chunks[-1] != b"IEND" or b"IDAT" not in chunks:
        raise ValueError("incomplete PNG structure")
    if strict_output:
        expected_bytes = dimensions[1] * (1 + 4 * dimensions[0])
        decoder = zlib.decompressobj()
        scanlines = decoder.decompress(b"".join(compressed), expected_bytes + 1)
        if len(scanlines) != expected_bytes or not decoder.eof or decoder.unused_data or decoder.unconsumed_tail:
            raise ValueError("incomplete zlib stream, trailing compressed payload or incorrect scanline length")
        stride = 1 + 4 * dimensions[0]
        if any(scanlines[offset] > 4 for offset in range(0, expected_bytes, stride)):
            raise ValueError("invalid PNG filter")
    return dimensions, [kind.decode("ascii") for kind in chunks]


def decode(raw):
    with Image.open(io.BytesIO(raw)) as image:
        if image.format != "PNG" or getattr(image, "n_frames", 1) != 1:
            raise ValueError("single-frame PNG required")
        return image.convert("RGBA")


def check_files(source, output, regions_path, *, normalized_source=False):
    # Explicit Rust-normalized synthetic JPEG anchors can exceed the source
    # intake cap. Default source verification retains the original 25MiB bound.
    src = bounded_read(source, (128 if normalized_source else 25) * 1024 * 1024)
    png = bounded_read(output, 128 * 1024 * 1024)
    dimensions, _ = inspect_png(src, False)
    output_dimensions, chunks = inspect_png(png, True)
    if dimensions != output_dimensions:
        raise ValueError("output dimensions differ from source")
    regions = json.loads(bounded_read(regions_path, 64 * 1024))
    if not isinstance(regions, list):
        raise ValueError("region list required")
    expected = decode(src)
    count = 0
    width, height = dimensions
    for region in regions:
        if not isinstance(region, list) or len(region) != 4 or any(type(v) is not int or not -(2**63) <= v < 2**63 for v in region):
            raise ValueError("regions must contain four signed integer coordinates")
        x0, y0, x1, y1 = region
        left, right = sorted((max(0, min(width, x0)), max(0, min(width, x1))))
        top, bottom = sorted((max(0, min(height, y0)), max(0, min(height, y1))))
        if left < right and top < bottom:
            expected.paste(COVER, (left, top, right, bottom))
            count += 1
    if not count:
        raise ValueError("no effective mask; source-only export forbidden")
    actual_pixels = decode(png).tobytes()
    if actual_pixels != expected.tobytes():
        raise ValueError("full-image pixels differ from independent mask composition")
    return {"ok": True, "dimensions": list(dimensions), "effective_regions": count,
            "rgba_sha256": hashlib.sha256(actual_pixels).hexdigest(),
            "png_sha256": hashlib.sha256(png).hexdigest(), "chunks": chunks}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--regions", type=Path, required=True)
    args = parser.parse_args()
    try:
        result = check_files(args.source, args.output, args.regions)
    except (ValueError, OSError, struct.error, zlib.error) as error:
        print(json.dumps({"ok": False, "reason": str(error)}))
        return 1
    print(json.dumps(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
