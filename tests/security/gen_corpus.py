"""Generate a local malformed/hostile intake corpus for ShareReady (M4 hardening).

Everything is synthesized here from Pillow, zlib and struct; nothing is
downloaded and no real screenshots are used. Output goes to a gitignored
folder (default: tests/security/corpus/) and is consumed by
crates/shareready-core/tests/intake_hardening.rs.

    python tests/security/gen_corpus.py [--out DIR]

File-name prefixes tell the Rust test which intake function to call:
    src_*  -> decode_source (PNG/JPEG user files)
    dib_*  -> dib_to_png    (clipboard CF_DIB / CF_DIBV5 payloads)
    gen_*  -> decode_generated_png (expected size encoded as gen_WxH_*)
"""
from __future__ import annotations

import argparse
import io
import struct
import zlib
from pathlib import Path

from PIL import Image

PNG_MAGIC = b"\x89PNG\r\n\x1a\n"
MAX_INPUT_BYTES = 25 * 1024 * 1024


# ---------------------------------------------------------------- PNG helpers
def chunk(kind: bytes, data: bytes, crc: int | None = None) -> bytes:
    if crc is None:
        crc = zlib.crc32(kind + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", crc)


def ihdr(w: int, h: int, depth: int = 8, color: int = 6, interlace: int = 0) -> bytes:
    return chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, depth, color, 0, 0, interlace))


def channels(color: int) -> int:
    return {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[color]


def row_bytes(w: int, depth: int, color: int) -> int:
    return (w * channels(color) * depth + 7) // 8


def raw_scanlines(w: int, h: int, depth: int, color: int, fill: int = 0x55) -> bytes:
    return b"".join(b"\x00" + bytes([fill]) * row_bytes(w, depth, color) for _ in range(h))


def adam7_scanlines(w: int, h: int, depth: int, color: int) -> bytes:
    passes = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4),
              (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)]
    out = b""
    for x0, y0, dx, dy in passes:
        pw = max(0, (w - x0 + dx - 1) // dx)
        ph = max(0, (h - y0 + dy - 1) // dy)
        if pw and ph:
            out += raw_scanlines(pw, ph, depth, color, fill=0x33 + x0 + y0)
    return out


def png(w: int, h: int, depth: int = 8, color: int = 6, interlace: int = 0,
        extra_before_idat: bytes = b"", scan: bytes | None = None) -> bytes:
    if scan is None:
        scan = adam7_scanlines(w, h, depth, color) if interlace else raw_scanlines(w, h, depth, color)
    return (PNG_MAGIC + ihdr(w, h, depth, color, interlace) + extra_before_idat
            + chunk(b"IDAT", zlib.compress(scan, 9)) + chunk(b"IEND", b""))


def pillow_bytes(img: Image.Image, fmt: str, **kw) -> bytes:
    out = io.BytesIO()
    img.save(out, fmt, **kw)
    return out.getvalue()


def gradient(w: int, h: int, mode: str = "RGB") -> Image.Image:
    img = Image.new("RGB", (w, h))
    img.putdata([((x * 7) & 255, (y * 13) & 255, (x ^ y) & 255) for y in range(h) for x in range(w)])
    return img.convert(mode)


# --------------------------------------------------------------- JPEG helpers
def jpeg_insert_app1(jpeg: bytes, payload: bytes) -> bytes:
    assert jpeg[:2] == b"\xff\xd8"
    seg_len = len(payload) + 2
    assert seg_len <= 0xFFFF
    return jpeg[:2] + b"\xff\xe1" + struct.pack(">H", seg_len) + payload + jpeg[2:]


def exif_tiff(entries: list[tuple[int, int, int, bytes]], little: bool = True,
              ifd_offset: int = 8, count_override: int | None = None) -> bytes:
    """entries: (tag, type, count, 4-byte value field)."""
    e = "<" if little else ">"
    head = (b"II" if little else b"MM") + struct.pack(e + "HI", 42, ifd_offset)
    n = len(entries) if count_override is None else count_override
    body = struct.pack(e + "H", n)
    for tag, typ, cnt, val in entries:
        body += struct.pack(e + "HHI", tag, typ, cnt) + val
    body += struct.pack(e + "I", 0)
    return b"Exif\x00\x00" + head + b"\x00" * (ifd_offset - 8 if ifd_offset < 4096 else 0) + body


def orientation_entry(value: int, little: bool = True, typ: int = 3, count: int = 1):
    e = "<" if little else ">"
    return (0x0112, typ, count, struct.pack(e + "HH", value & 0xFFFF, 0))


def find_marker(jpeg: bytes, marker: int) -> int:
    i = jpeg.find(bytes([0xFF, marker]))
    assert i >= 0, hex(marker)
    return i


# ---------------------------------------------------------------- DIB helpers
def dib(header: int = 40, w: int = 4, h: int = 4, planes: int = 1, bits: int = 24,
        compression: int = 0, clr_used: int = 0, cs_type: int = 0x73524742,
        masks=(0x00FF0000, 0x0000FF00, 0x000000FF, 0), pixel_bytes: int | None = None,
        profile=(0, 0)) -> bytes:
    hdr = bytearray(header)
    struct.pack_into("<Iii", hdr, 0, header, w, h)
    struct.pack_into("<HHI", hdr, 12, planes, bits, compression)
    struct.pack_into("<I", hdr, 32, clr_used)
    out = bytes(hdr)
    if header >= 108:
        hdr = bytearray(out)
        struct.pack_into("<IIIII", hdr, 40, *masks, cs_type)
        if header == 124:
            struct.pack_into("<II", hdr, 112, *profile)
        out = bytes(hdr)
    elif header == 40 and compression == 3:
        out += struct.pack("<III", *masks[:3])
    if pixel_bytes is None:
        bpp = max(bits, 1)
        stride = ((abs(w) * bpp + 31) // 32) * 4 if abs(w) < 1 << 16 else 0
        pixel_bytes = min(stride * min(abs(h), 1 << 12), 1 << 20)
    return out + bytes((i * 37) & 255 for i in range(pixel_bytes))


# ------------------------------------------------------------------- corpus
def build() -> dict[str, bytes]:
    c: dict[str, bytes] = {}
    good_png = pillow_bytes(gradient(32, 24, "RGBA"), "PNG")
    good_jpg = pillow_bytes(gradient(32, 24), "JPEG", quality=85)

    # --- PNG: decompression bombs and dimension abuse
    c["src_png_bomb_100000x100000.png"] = png(100_000, 100_000, scan=b"\x00" * 64)
    c["src_png_bomb_u32max.png"] = png(0xFFFFFFFF, 0xFFFFFFFF, scan=b"\x00")
    c["src_png_bomb_2pow31.png"] = png(1 << 31, 1, scan=b"\x00")
    c["src_png_dims_24mp_plus1.png"] = png(24_000_001, 1, scan=b"\x00")
    c["src_png_dims_zero_width.png"] = png(0, 16, scan=b"\x00")
    # Within the 24 MP gate, but tiny truncated IDAT: decoder must fail, not hang.
    c["src_png_24mp_tiny_idat.png"] = png(4898, 4898, scan=b"\x00" * 64)
    # Deflate bomb: 1x1 image whose IDAT inflates to 64 MiB of zeros (~64 KiB on disk).
    c["src_png_deflate_bomb_1x1.png"] = png(1, 1, scan=b"\x00" * (64 << 20))
    # Highly compressible but legitimate 16 MP image (~16 KiB on disk).
    c["src_png_zeros_4000x4000.png"] = png(4000, 4000, depth=8, color=0,
                                           scan=b"\x00" * (4001 * 4000))

    # --- PNG: structure errors
    c["src_png_truncated_half.png"] = good_png[: len(good_png) // 2]
    c["src_png_truncated_magic_only.png"] = PNG_MAGIC
    c["src_png_truncated_in_ihdr.png"] = good_png[:20]
    c["src_png_no_iend.png"] = good_png[:-12]
    bad = bytearray(good_png)
    bad[29] ^= 0xFF  # IHDR CRC
    c["src_png_bad_crc_ihdr.png"] = bytes(bad)
    idat = good_png.find(b"IDAT")
    idat_len = struct.unpack(">I", good_png[idat - 4: idat])[0]
    bad = bytearray(good_png)
    bad[idat + 4 + idat_len] ^= 0xFF  # IDAT CRC
    c["src_png_bad_crc_idat.png"] = bytes(bad)
    bad = bytearray(good_png)
    bad[idat + 10] ^= 0xFF  # corrupt deflate stream, CRC recomputed below
    body = bytes(bad[idat: idat + 4 + idat_len])
    bad[idat + 4 + idat_len: idat + 8 + idat_len] = struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
    c["src_png_corrupt_zlib_good_crc.png"] = bytes(bad)
    c["src_png_chunk_before_ihdr.png"] = (PNG_MAGIC + chunk(b"tEXt", b"k\x00v")
                                         + good_png[8:])
    c["src_png_idat_before_ihdr.png"] = (PNG_MAGIC + chunk(b"IDAT", zlib.compress(b"\x00" * 5))
                                        + good_png[8:])
    c["src_png_two_ihdr.png"] = (PNG_MAGIC + ihdr(2, 2) + ihdr(50_000, 50_000)
                                 + chunk(b"IDAT", zlib.compress(raw_scanlines(2, 2, 8, 6)))
                                 + chunk(b"IEND", b""))
    c["src_png_chunk_len_huge.png"] = PNG_MAGIC + ihdr(2, 2) + struct.pack(">I", 0x7FFFFFFF) + b"IDAT\x00\x00"
    c["src_png_bad_depth_color.png"] = png(2, 2, depth=7, color=6, scan=b"\x00" * 16)
    c["src_png_bad_color_type.png"] = png(2, 2, depth=8, color=5, scan=b"\x00" * 16)
    c["src_png_bad_interlace_method.png"] = png(2, 2, interlace=2, scan=b"\x00" * 18)
    c["src_png_unknown_critical_chunk.png"] = png(2, 2, extra_before_idat=chunk(b"ABCD", b"x"))
    c["src_png_trailing_garbage.png"] = good_png + b"\x00" * 4096

    # --- PNG: legal-but-unusual formats
    frames = [gradient(16, 12, "RGBA"), Image.new("RGBA", (16, 12), (255, 0, 0, 128))]
    c["src_png_apng_two_frames.png"] = pillow_bytes(frames[0], "PNG", save_all=True,
                                                     append_images=frames[1:], duration=50, loop=0)
    c["src_png_16bit_rgba.png"] = png(8, 8, depth=16, color=6)
    c["src_png_16bit_gray.png"] = pillow_bytes(Image.new("I;16", (8, 8), 40000), "PNG")
    c["src_png_1bit_gray.png"] = png(9, 5, depth=1, color=0)
    c["src_png_palette.png"] = pillow_bytes(gradient(16, 12).convert("P"), "PNG")
    plte = chunk(b"PLTE", bytes(range(6)))  # 2 entries only
    c["src_png_palette_index_oob.png"] = png(4, 4, depth=8, color=3, extra_before_idat=plte,
                                             scan=b"".join(b"\x00" + b"\xff" * 4 for _ in range(4)))
    c["src_png_palette_missing_plte.png"] = png(4, 4, depth=8, color=3)
    c["src_png_palette_trns_too_long.png"] = png(4, 4, depth=8, color=3, extra_before_idat=plte
                                                 + chunk(b"tRNS", b"\x80" * 255))
    c["src_png_interlaced_rgba.png"] = png(13, 11, interlace=1)
    c["src_png_interlaced_16bit.png"] = png(9, 7, depth=16, color=2, interlace=1)
    c["src_png_interlaced_truncated.png"] = png(13, 11, interlace=1)[:-40]
    c["src_png_huge_text_chunk.png"] = png(2, 2, extra_before_idat=chunk(b"zTXt", b"k\x00\x00"
                                                                        + zlib.compress(b"A" * (16 << 20))))
    c["src_png_huge_iccp.png"] = png(2, 2, extra_before_idat=chunk(b"iCCP", b"p\x00\x00"
                                                                  + zlib.compress(b"\x00" * (32 << 20))))

    # --- JPEG
    c["src_jpg_truncated_half.jpg"] = good_jpg[: len(good_jpg) // 2]
    c["src_jpg_truncated_soi_only.jpg"] = good_jpg[:3]
    c["src_jpg_truncated_no_eoi.jpg"] = good_jpg[:-2]
    c["src_jpg_cmyk.jpg"] = pillow_bytes(gradient(16, 12, "CMYK"), "JPEG")
    c["src_jpg_grayscale.jpg"] = pillow_bytes(gradient(16, 12, "L"), "JPEG")
    c["src_jpg_progressive.jpg"] = pillow_bytes(gradient(64, 48), "JPEG", progressive=True)
    c["src_jpg_progressive_truncated.jpg"] = pillow_bytes(gradient(64, 48), "JPEG", progressive=True)[:300]
    c["src_jpg_subsampling_444.jpg"] = pillow_bytes(gradient(17, 9), "JPEG", subsampling=0)
    c["src_jpg_valid_restart_markers.jpg"] = pillow_bytes(gradient(64, 48), "JPEG", restart_marker_blocks=1)
    # Well-formed, but zune-jpeg 0.4.21 rejects progressive + restart intervals (known limit).
    c["src_jpg_progressive_restart.jpg"] = pillow_bytes(gradient(64, 48), "JPEG", progressive=True,
                                                         restart_marker_rows=1)
    c["src_jpg_valid_trailing_after_eoi.jpg"] = good_jpg + b"trailer\xff\xd8\x00" * 8
    c["src_jpg_valid_fill_bytes_before_eoi.jpg"] = good_jpg[:-2] + b"\xff\xff\xff\xd9"
    # Progressive JPEG cut after its first scan: zune-jpeg 0.4.21 looped forever here.
    prog = pillow_bytes(gradient(64, 48), "JPEG", progressive=True)
    first_scan = find_marker(prog, 0xDA)
    second_scan = prog.find(b"\xff\xda", first_scan + 2)
    c["src_jpg_progressive_cut_after_scan1.jpg"] = prog[:second_scan]
    c["src_jpg_progressive_cut_dangling_marker.jpg"] = prog[:second_scan] + b"\xff\xff\xff\xff"
    sos = find_marker(good_jpg, 0xDA)
    bad = bytearray(good_jpg)
    for k in range(sos + 20, len(bad) - 2, 7):
        bad[k] ^= 0x5A
    c["src_jpg_corrupt_scan.jpg"] = bytes(bad)
    dht = find_marker(good_jpg, 0xC4)
    bad = bytearray(good_jpg)
    bad[dht + 5: dht + 21] = b"\xff" * 16  # Huffman code-length counts overflow
    c["src_jpg_corrupt_huffman.jpg"] = bytes(bad)
    dqt = find_marker(good_jpg, 0xDB)
    bad = bytearray(good_jpg)
    bad[dqt + 4] = 0x0F  # invalid quant table id/precision
    c["src_jpg_corrupt_dqt.jpg"] = bytes(bad)
    sof = find_marker(good_jpg, 0xC0)
    for name, (h, w) in {"zero": (0, 0), "65535": (0xFFFF, 0xFFFF),
                         "over24mp": (5000, 5000), "edge24mp": (4000, 6000)}.items():
        bad = bytearray(good_jpg)
        struct.pack_into(">HH", bad, sof + 5, h, w)
        c[f"src_jpg_sof_dims_{name}.jpg"] = bytes(bad)
    bad = bytearray(good_jpg)
    bad[sof + 9] = 0  # zero components
    c["src_jpg_sof_zero_components.jpg"] = bytes(bad)
    bad = bytearray(good_jpg)
    bad[sof + 2: sof + 4] = b"\xff\xff"  # segment length past EOF
    c["src_jpg_sof_len_past_eof.jpg"] = bytes(bad)
    c["src_jpg_two_sof.jpg"] = good_jpg[:sof] + good_jpg[sof: sof + 19].replace(
        good_jpg[sof + 5: sof + 9], struct.pack(">HH", 4000, 6000)) + good_jpg[sof:]
    c["src_jpg_arith_sof9.jpg"] = good_jpg[:sof + 1] + b"\xc9" + good_jpg[sof + 2:]
    c["src_jpg_lossless_sof3.jpg"] = good_jpg[:sof + 1] + b"\xc3" + good_jpg[sof + 2:]

    # --- JPEG EXIF orientation abuse (APP1 inserted right after SOI)
    for v in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 255, 0xFFFF]:
        c[f"src_jpg_exif_orient_{v}.jpg"] = jpeg_insert_app1(good_jpg, exif_tiff([orientation_entry(v)]))
    c["src_jpg_exif_orient_6_bigendian.jpg"] = jpeg_insert_app1(
        good_jpg, exif_tiff([orientation_entry(6, little=False)], little=False))
    c["src_jpg_exif_orient_wrong_type.jpg"] = jpeg_insert_app1(good_jpg, exif_tiff([orientation_entry(6, typ=4)]))
    c["src_jpg_exif_orient_count_2.jpg"] = jpeg_insert_app1(good_jpg, exif_tiff([orientation_entry(6, count=2)]))
    c["src_jpg_exif_ifd_offset_max.jpg"] = jpeg_insert_app1(good_jpg, exif_tiff([], ifd_offset=0xFFFFFFFF))
    c["src_jpg_exif_ifd_count_claims_65535.jpg"] = jpeg_insert_app1(
        good_jpg, exif_tiff([orientation_entry(6)], count_override=0xFFFF))
    c["src_jpg_exif_ifd_count_513.jpg"] = jpeg_insert_app1(
        good_jpg, exif_tiff([(0x010F, 2, 4, b"abc\x00")] * 512 + [orientation_entry(6)]))
    c["src_jpg_exif_ifd_count_512.jpg"] = jpeg_insert_app1(
        good_jpg, exif_tiff([(0x010F, 2, 4, b"abc\x00")] * 511 + [orientation_entry(6)]))
    c["src_jpg_exif_bad_tiff_magic.jpg"] = jpeg_insert_app1(good_jpg, b"Exif\x00\x00II\x2b\x00\x08\x00\x00\x00")
    c["src_jpg_exif_truncated_tiff.jpg"] = jpeg_insert_app1(good_jpg, b"Exif\x00\x00II\x2a")
    # Oversized EXIF: a maximal 64 KiB APP1 plus many repeated APP1 segments (~4 MiB).
    big = exif_tiff([orientation_entry(8)]) + b"\x00" * (0xFFFF - 2 - 40)
    c["src_jpg_exif_max_segment.jpg"] = jpeg_insert_app1(good_jpg, big[: 0xFFFF - 2])
    many = good_jpg
    for _ in range(64):
        many = jpeg_insert_app1(many, big[: 0xFFFF - 2])
    c["src_jpg_exif_64_max_segments.jpg"] = many
    c["src_jpg_app1_len_lt2.jpg"] = good_jpg[:2] + b"\xff\xe1\x00\x01" + good_jpg[2:]
    c["src_jpg_app1_len_past_eof.jpg"] = good_jpg[:2] + b"\xff\xe1\xff\xff" + b"Exif\x00\x00II*\x00"

    # --- size limits
    c["src_empty_0_bytes.png"] = b""
    c["src_over_25mib_png_magic.png"] = good_png + b"\x00" * (MAX_INPUT_BYTES + 1 - len(good_png))
    c["src_over_25mib_jpeg_magic.jpg"] = good_jpg[:-2] + b"\x00" * (MAX_INPUT_BYTES + 1 - len(good_jpg)) + b"\xff\xd9"
    c["src_exactly_25mib_png_trailing.png"] = good_png + b"\x00" * (MAX_INPUT_BYTES - len(good_png))
    c["src_wrong_magic_gif.gif"] = pillow_bytes(gradient(8, 8), "GIF")
    c["src_wrong_magic_bmp.bmp"] = pillow_bytes(gradient(8, 8), "BMP")
    c["src_wrong_magic_webp.webp"] = b"RIFF\x10\x00\x00\x00WEBPVP8 " + b"\x00" * 8

    # --- clipboard DIB headers
    c["dib_valid_24bit_bottom_up.dib"] = dib()
    c["dib_valid_32bit_v5_alpha.dib"] = dib(124, bits=32, compression=3,
                                           masks=(0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000))
    c["dib_empty.dib"] = b""
    c["dib_39_bytes.dib"] = dib()[:39]
    c["dib_header_only.dib"] = dib(pixel_bytes=0)
    for header in [0, 12, 39, 41, 64, 107, 125, 0xFFFFFFFF]:
        raw = bytearray(dib())
        struct.pack_into("<I", raw, 0, header)
        c[f"dib_header_size_{header}.dib"] = bytes(raw)
    for name, (w, h) in {
        "w_i32max": (0x7FFFFFFF, 1), "h_i32max": (1, 0x7FFFFFFF), "h_i32min": (4, -0x80000000),
        "w_negative": (-4, 4), "w_zero": (0, 4), "h_zero": (4, 0), "65536_sq": (65536, 65536),
        "24mp_plus1": (24_000_001, 1), "neg_24mp_plus1": (4899, -4899), "both_i32max": (0x7FFFFFFF, 0x7FFFFFFF),
        "w_i32min": (-0x80000000, 1),
    }.items():
        c[f"dib_dims_{name}.dib"] = dib(w=w, h=h, pixel_bytes=4096)
    # Within limits but pixel data truncated by one byte / one row.
    full = dib(w=7, h=-5)
    c["dib_truncated_last_byte.dib"] = full[:-1]
    c["dib_truncated_last_row.dib"] = full[:-24]
    c["dib_24mp_claimed_no_pixels.dib"] = dib(w=6000, h=4000, pixel_bytes=16)
    for bits in [0, 1, 4, 8, 15, 16, 48, 64, 0xFFFF]:
        c[f"dib_bits_{bits}.dib"] = dib(bits=bits)
    for planes in [0, 2, 0xFFFF]:
        c[f"dib_planes_{planes}.dib"] = dib(planes=planes)
    for comp in [1, 2, 4, 5, 6, 0xFFFFFFFF]:
        c[f"dib_compression_{comp}.dib"] = dib(bits=32, compression=comp)
    c["dib_bitfields_24bit.dib"] = dib(bits=24, compression=3)
    c["dib_bitfields_40_truncated_masks.dib"] = dib(bits=32, compression=3, pixel_bytes=0)[:46]
    c["dib_bitfields_bgr_masks.dib"] = dib(bits=32, compression=3, masks=(0xFF, 0xFF00, 0xFF0000, 0))
    c["dib_clr_used_huge.dib"] = dib(clr_used=0xFFFFFFFF)
    c["dib_v5_profile_linked.dib"] = dib(124, bits=32, compression=3, cs_type=0x4C494E4B,
                                         profile=(0xFFFFFFF0, 0xFFFFFFFF))
    c["dib_v5_profile_offsets.dib"] = dib(124, bits=32, compression=3, profile=(124, 0x7FFFFFFF))
    c["dib_v4_alpha_mask_odd.dib"] = dib(108, bits=32, compression=3,
                                        masks=(0x00FF0000, 0x0000FF00, 0x000000FF, 0x0000FF00))
    c["dib_v5_cs_type_zero.dib"] = dib(124, cs_type=0)

    # --- generated-artifact readback (expected size in the name)
    gen = pillow_bytes(Image.new("RGBA", (5, 3), (1, 2, 3, 255)), "PNG")
    c["gen_5x3_valid.png"] = gen
    c["gen_5x3_with_text.png"] = pillow_bytes(Image.new("RGBA", (5, 3)), "PNG",
                                              pnginfo=_pnginfo())
    c["gen_5x3_apng.png"] = pillow_bytes(Image.new("RGBA", (5, 3)), "PNG", save_all=True,
                                         append_images=[Image.new("RGBA", (5, 3), (9, 9, 9, 9))])
    c["gen_5x3_interlaced.png"] = png(5, 3, interlace=1)
    c["gen_5x3_rgb.png"] = pillow_bytes(Image.new("RGB", (5, 3)), "PNG")
    c["gen_5x3_deflate_bomb.png"] = png(5, 3, scan=b"\x00" * (64 << 20))
    c["gen_4898x4898_tiny_idat.png"] = png(4898, 4898, scan=b"\x00" * 64)
    c["gen_5x3_truncated.png"] = gen[:-13] + gen[-12:]
    return c


def _pnginfo():
    from PIL import PngImagePlugin
    info = PngImagePlugin.PngInfo()
    info.add_text("Author", "synthetic")
    return info


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", type=Path, default=Path(__file__).resolve().parent / "corpus")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    corpus = build()
    for name, data in sorted(corpus.items()):
        (args.out / name).write_bytes(data)
    total = sum(len(d) for d in corpus.values())
    print(f"wrote {len(corpus)} files ({total / 1048576:.1f} MiB) to {args.out}")


if __name__ == "__main__":
    main()
