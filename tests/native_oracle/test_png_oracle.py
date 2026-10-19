"""Independent file-level contract: incorrect output must be rejected, not merely decoded."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import struct
import zlib

from PIL import Image, PngImagePlugin

ROOT = Path(__file__).resolve().parents[2]
ORACLE = Path(__file__).with_name("png_oracle.py")
BASE = bytes((10, 20, 30, 255))
COVER = bytes((17, 24, 39, 255))
# Four-by-three image; the two middle pixels of the middle row are covered.
EXPECTED = BASE * 5 + COVER * 2 + BASE * 5


class PngOracleTests(unittest.TestCase):
    def setUp(self):
        parent = ROOT / "target/native-oracle-tests"
        parent.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=parent)
        self.addCleanup(self.temp.cleanup)
        self.dir = Path(self.temp.name)
        self.source = self.dir / "source.png"
        self.output = self.dir / "output.png"
        self.regions = self.dir / "regions.json"
        Image.frombytes("RGBA", (4, 3), BASE * 12).save(self.source)
        Image.frombytes("RGBA", (4, 3), EXPECTED).save(self.output)
        self.regions.write_text(json.dumps([[1, 1, 3, 2]]), encoding="utf-8")

    def check_output(self, expected_exit, timeout=15):
        result = subprocess.run([sys.executable, str(ORACLE), "--source", str(self.source),
                                 "--output", str(self.output), "--regions", str(self.regions)],
                                capture_output=True, text=True, encoding="utf-8", timeout=timeout)
        self.assertEqual(result.returncode, expected_exit, result.stdout + result.stderr)
        return json.loads(result.stdout)

    def rewrite_idat(self, transform):
        raw = self.output.read_bytes()
        pos, chunks = 8, []
        while pos < len(raw):
            size = struct.unpack_from(">I", raw, pos)[0]
            chunks.append((raw[pos + 4:pos + 8], raw[pos + 8:pos + 8 + size]))
            pos += size + 12
        payload = transform(b"".join(body for kind, body in chunks if kind == b"IDAT"))
        rebuilt, emitted = bytearray(raw[:8]), False
        for kind, body in chunks:
            if kind == b"IDAT":
                if emitted:
                    continue
                body, emitted = payload, True
            rebuilt.extend(struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body)))
        self.output.write_bytes(rebuilt)

    def test_exact_mask_reports_independently_derived_pixel_hash(self):
        report = self.check_output(0)
        self.assertTrue(report["ok"])
        self.assertEqual(report["rgba_sha256"], hashlib.sha256(EXPECTED).hexdigest())
        self.assertEqual(report["dimensions"], [4, 3])

    def test_incomplete_coverage_cannot_export_source(self):
        self.output.write_bytes(self.source.read_bytes())
        self.assertFalse(self.check_output(1)["ok"])

    def test_covering_an_extra_unselected_pixel_is_rejected(self):
        Image.frombytes("RGBA", (4, 3), BASE * 5 + COVER * 3 + BASE * 4).save(self.output)
        self.assertFalse(self.check_output(1)["ok"])

    def test_modifying_uncovered_pixels_is_rejected(self):
        changed = bytes((99, 20, 30, 255)) + EXPECTED[4:]
        Image.frombytes("RGBA", (4, 3), changed).save(self.output)
        self.assertFalse(self.check_output(1)["ok"])

    def test_dimensions_must_match_source(self):
        Image.new("RGBA", (3, 4), tuple(COVER)).save(self.output)
        self.assertFalse(self.check_output(1)["ok"])

    def test_source_metadata_cannot_survive_output(self):
        metadata = PngImagePlugin.PngInfo()
        metadata.add_text("private", "synthetic marker")
        Image.frombytes("RGBA", (4, 3), EXPECTED).save(self.output, pnginfo=metadata)
        self.assertFalse(self.check_output(1)["ok"])

    def test_payload_after_iend_is_rejected(self):
        self.output.write_bytes(self.output.read_bytes() + b"synthetic-source-payload")
        self.assertFalse(self.check_output(1)["ok"])

    def test_invalid_chunk_crc_is_rejected(self):
        raw = bytearray(self.output.read_bytes())
        raw[29] ^= 1  # IHDR CRC, rather than a decoded-pixel difference.
        self.output.write_bytes(raw)
        self.assertFalse(self.check_output(1)["ok"])

    def test_truncated_chunk_is_rejected(self):
        self.output.write_bytes(self.output.read_bytes()[:-7])
        self.assertFalse(self.check_output(1)["ok"])

    def test_empty_or_ineffective_regions_cannot_validate_source_export(self):
        self.output.write_bytes(self.source.read_bytes())
        for regions in ([], [[1, 1, 1, 2]], [[10, 10, 20, 20]]):
            with self.subTest(regions=regions):
                self.regions.write_text(json.dumps(regions), encoding="utf-8")
                self.assertFalse(self.check_output(1)["ok"])

    def test_reverse_drag_is_normalized_without_changing_expected_pixels(self):
        self.regions.write_text(json.dumps([[3, 2, 1, 1]]), encoding="utf-8")
        self.assertTrue(self.check_output(0)["ok"])

    def test_fractional_regions_are_not_silently_truncated(self):
        self.regions.write_text(json.dumps([[1.1, 1, 3, 2]]), encoding="utf-8")
        self.assertFalse(self.check_output(1)["ok"])

    def test_payload_inside_idat_after_zlib_stream_is_rejected(self):
        self.rewrite_idat(lambda payload: payload + b"synthetic-private-source")
        self.assertFalse(self.check_output(1)["ok"])

    def test_second_zlib_stream_inside_idat_is_rejected(self):
        self.rewrite_idat(lambda payload: payload + zlib.compress(b"synthetic-private-source"))
        self.assertFalse(self.check_output(1)["ok"])

    def test_extra_decoded_scanline_bytes_are_rejected(self):
        self.rewrite_idat(lambda payload: zlib.compress(zlib.decompress(payload) + b"synthetic-private-source"))
        self.assertFalse(self.check_output(1)["ok"])

    def test_many_source_ancillary_chunks_do_not_cause_quadratic_stall(self):
        raw = self.source.read_bytes()
        chunk = struct.pack(">I", 0) + b"npAD" + struct.pack(">I", zlib.crc32(b"npAD"))
        self.source.write_bytes(raw[:33] + chunk * 40_000 + raw[33:])
        try:
            self.assertTrue(self.check_output(0, timeout=5)["ok"])
        except subprocess.TimeoutExpired:
            self.fail("A sub-megabyte PNG with ancillary chunks exceeded the bounded validation time")

    def test_nonuniform_alpha_pixels_outside_clipped_reversed_masks_are_preserved(self):
        pixels = bytes((9, 7, 5, 0, 3, 2, 1, 64, 80, 90, 100, 128, 11, 12, 13, 255))
        Image.frombytes("RGBA", (2, 2), pixels).save(self.source)
        # Reversed and clipped rectangle covers only the bottom-left pixel; overlap is harmless.
        self.regions.write_text(json.dumps([[1, 3, -2, 1], [0, 1, 1, 2]]), encoding="utf-8")
        expected = pixels[:8] + COVER + pixels[12:]
        Image.frombytes("RGBA", (2, 2), expected).save(self.output)
        self.assertEqual(self.check_output(0)["rgba_sha256"], hashlib.sha256(expected).hexdigest())

    def test_cover_must_have_exact_rgb_and_opaque_alpha(self):
        for bad in (bytes((17, 24, 38, 255)), bytes((17, 24, 39, 254))):
            with self.subTest(cover=bad):
                Image.frombytes("RGBA", (4, 3), BASE * 5 + bad * 2 + BASE * 5).save(self.output)
                self.assertFalse(self.check_output(1)["ok"])

    def test_rgb_output_cannot_hide_a_format_contract_change(self):
        Image.frombytes("RGBA", (4, 3), EXPECTED).convert("RGB").save(self.output)
        self.assertFalse(self.check_output(1)["ok"])


if __name__ == "__main__":
    unittest.main()
