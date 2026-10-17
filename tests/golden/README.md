# SR-V2-00: V1.3 synthetic golden images

This contract freezes all five pre-existing synthetic fixture SHA256 values, exact half-open redaction rectangles and RGBA full-frame output hashes. Run the real Rust V1 CLI and independently compare the decoded image with Pillow EXIF-normalized pixel truth; PNG structure and CRC are also validated.

Reproduce from root, with Pillow==12.3.0 installed:

    cargo build --release --locked --manifest-path e1-spike/Cargo.toml
    python tests/golden/verify_v1_baseline.py

A capture-mode execution is diagnostic only and cannot count as a frozen verification. Default strict mode must match committed expected_pixel_sha256 for all five. Report: /tmp/shareready-v2-00/golden-report.json (CI Linux). Do not use private user images in this fixture set. Clipboard and native GUI verification are separate.
T05 is a lossy JPEG: independently decoding the source with Pillow does not exactly match Rust image-rs. The test-only example e1-spike/examples/v1_baseline_preview.rs anchors source pixels to real V1 Rust decode; Pillow independently applies masks and checks the full exported PNG. A JPEG decoding-library change requires explicit compatibility review.
