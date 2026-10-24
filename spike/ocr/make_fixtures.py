"""Generate synthetic OCR fixtures for the Windows.Media.Ocr spike.

Usage: python spike/ocr/make_fixtures.py
Writes spike/ocr/fixtures/: <set>_<px>px_<scale>.png (review copy), matching .bgra
(u32 LE w, u32 LE h, BGRA rows) and truth.json. All text is invented; no real data.
Rendered with Pillow grayscale antialiasing (no ClearType subpixel rendering).
"""
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
FONT = REPO / "assets/fonts/NotoSansCJKsc-Regular.otf"
OUT = HERE / "fixtures"

SETS = {
    "zh": [
        "截图已保存到图片文件夹",
        "请选择需要遮挡的区域",
        "导出失败，请检查磁盘空间后重试",
        "复制到剪贴板成功",
        "撤销上一步操作",
        "打开最近使用的文件",
        "确定要删除这条标注吗",
        "设置快捷键与默认保存位置",
    ],
    "en": [
        "Screenshot saved to Pictures",
        "Select the area you want to hide",
        "Export failed. Check free disk space and retry.",
        "Copied to clipboard",
        "Undo the last action",
        "Open a recent file",
        "Delete this annotation?",
        "Shortcut keys and default save folder",
    ],
    "mixed": [
        "已导出 PNG 文件，大小 1.2 MB",
        "按 Ctrl+Shift+A 开始截图",
        "版本 0.3.0 更新说明",
        "无法连接到 Wi-Fi 网络",
        "订单号 A20261024 已发货",
        "使用 Esc 键取消当前选择",
        "共 12 个标注，3 处遮挡",
        "请在 Settings 中开启 OCR",
    ],
}
SIZES = [13, 14, 15, 16]
SCALES = [1.0, 1.25, 1.5]

# Ground truth of spike/assets/shot.png, copied from spike/make_shot.py.
SHOT_TRUTH = [
    "客服 · 小林",
    "您好，请提供订单号和注册手机号，", "我帮您查询退款进度。",
    "订单号 2026101908812，手机 138 0013 8000",
    "收到。退款已于今天 14:05 原路退回，", "预计 1–3 个工作日到账。",
    "如需凭证，可在 “我的 → 订单 → 退款详情” 查看。",
    "好的，谢谢！",
]


def write_bgra(img, path):
    rgba = img.convert("RGBA")
    r, g, b, a = rgba.split()
    data = Image.merge("RGBA", (b, g, r, a)).tobytes()
    w, h = rgba.size
    path.write_bytes(w.to_bytes(4, "little") + h.to_bytes(4, "little") + data)


def render(lines, px):
    font = ImageFont.truetype(str(FONT), px)
    pad, step = round(px * 1.2), round(px * 1.8)
    probe = ImageDraw.Draw(Image.new("RGB", (1, 1)))
    w = int(max(probe.textlength(t, font=font) for t in lines)) + 2 * pad
    h = step * len(lines) + 2 * pad
    img = Image.new("RGB", (w, h), "#ffffff")
    d = ImageDraw.Draw(img)
    for i, t in enumerate(lines):
        d.text((pad, pad + step * i), t, font=font, fill="#222222")
    return img


def main():
    OUT.mkdir(exist_ok=True)
    truth = {}
    for name, lines in SETS.items():
        for size in SIZES:
            for scale in SCALES:
                px = round(size * scale)
                stem = f"{name}_{size}px_{int(scale * 100)}"
                img = render(lines, px)
                img.save(OUT / f"{stem}.png")
                write_bgra(img, OUT / f"{stem}.bgra")
                truth[stem] = {"set": name, "size": size, "scale": scale, "px": px, "text": lines}
    shot = Image.open(REPO / "spike/assets/shot.png")
    write_bgra(shot, OUT / "shot.bgra")
    truth["shot"] = {"set": "shot", "size": 13, "scale": 1.0, "px": 13, "text": SHOT_TRUTH}
    # 600x400 latency region: top-left crop of the synthetic chat (all five bubbles).
    crop = shot.crop((10, 0, 610, 400))
    crop.save(OUT / "region_600x400.png")
    write_bgra(crop, OUT / "region_600x400.bgra")
    (OUT / "truth.json").write_text(json.dumps(truth, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"{len(truth)} fixtures + region_600x400 in {OUT}")


if __name__ == "__main__":
    main()
