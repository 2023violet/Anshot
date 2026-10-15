# ShareReady E2 Workbench · isolated engineering preview

**This is a real Rust/WASM-integrated user interface, not the earlier simulation-only visual mock. It is NOT a privacy-certified or release-ready application.**

This branch and Draft PR #12 are based on the E1 engineering branch; they do not change main, merge E1, override the unresolved E1e security audit, or imply Stage 01 demand validation.

## What is implemented

A single-image editing workspace reflecting the accepted V1 visual direction: local file chooser/drop/paste, draw and select/move masks, bottom-right resize handle, region inspector, undo/redo, fit/zoom/pan, real Rust-generated PNG review, Clipboard write status and download fallback. The preview contains **no external tracking scripts, CDN, cloud storage or upload backend**.

**For development, input is deliberately locked to the SHA-256 checksums of four synthetic fixture images** T01_CHAT, T02_ORDER, T04_EDGES and T05_JPEG_EXIF. An arbitrary private screenshot is rejected even if it is a valid PNG or JPEG; this guard is intentional until E1e independent safety review.

## Running it locally (from repository root)

Requires Rust, rustup, Python 3 and wasm-bindgen-cli 0.2.105. Never use real/private images for this preview.

    rustup target add wasm32-unknown-unknown
    cargo fetch --manifest-path e1-spike/Cargo.toml --locked
    cargo build --manifest-path e1-spike/Cargo.toml --locked --offline --release --target wasm32-unknown-unknown --features web
    cargo install wasm-bindgen-cli --version 0.2.105 --locked
    mkdir -p e1-spike/web/pkg
    wasm-bindgen --target web --out-name shareready_e1 --out-dir e1-spike/web/pkg e1-spike/target/wasm32-unknown-unknown/release/shareready_e1.wasm
    python3 -m http.server 8000 --bind 127.0.0.1

Open http://127.0.0.1:8000/e2-workbench/index.html and choose **打开合成示例**. Localhost is required for WebAssembly and Clipboard APIs; do not open the HTML directly as a file.

## Running the real end-to-end test

After building the WASM module above:

    python3 -m pip install Pillow==12.3.0 playwright==1.55.0
    python3 e2-workbench/tests/verify_e2_browser.py

The test loads all four frozen synthetic images in Chromium, draws source-pixel masks through real mouse events, moves and resizes a region, restores geometry with Undo, tests zoom/fit, reencodes via actual Rust WASM, redownloads and decodes all four PNGs, verifies target pixel rectangles and keep regions, tests unknown-image rejection, old export invalidation, local-only network requests and real clipboard-denied fallback.

The CI workflow e2-workbench-integration.yml regenerates the WASM module and reruns the native Rust tests from the exact E2 branch; accelerated UI CI separately uses the previously SHA-grounded E1 WASM binary to shorten feedback loops.

## Do not overclaim

- This version does not scan a private image to find PII or guarantee that the user selected enough regions.
- Copying may succeed without an independently verified clipboard readback; its on-screen wording is deliberately limited.
- E1e independent security and license review is pending, as are keyboard-only mask movement/resizing, real-user feedback, multi-browser accessibility and performance benchmarks.
- No public website, release, product merge or permission to handle personal images has been granted.
