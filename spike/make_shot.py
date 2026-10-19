"""Generate the synthetic chat screenshot shared by all Gate 03 framework spikes.

Usage: python -I spike/make_shot.py
Writes spike/assets/shot.png (620x430). Content is synthetic; no real data.
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent
FONT = ROOT.parent / "poc/native-egui/assets/fonts/NotoSansCJKsc-Regular.otf"
OUT = ROOT / "assets/shot.png"

W, H = 620, 430
img = Image.new("RGB", (W, H), "#f3f5f8")
d = ImageDraw.Draw(img)
f13 = ImageFont.truetype(str(FONT), 13)
f14 = ImageFont.truetype(str(FONT), 14)

d.rectangle([0, 0, W, 44], fill="#ffffff")
d.line([0, 44, W, 44], fill="#e3e5e8")
d.ellipse([16, 8, 44, 36], fill="#c9d4e4")
d.text((54, 13), "客服 · 小林", font=f14, fill="#222222")


def bubble(x, y, lines, me=False, right=False):
    tw = max(d.textlength(t, font=f13) for t in lines)
    bw, bh = int(tw) + 24, 20 * len(lines) + 18
    if right:
        x = W - 16 - bw
    fill, edge = ("#95ec69", "#86d65e") if me else ("#ffffff", "#e6e8eb")
    d.rounded_rectangle([x, y, x + bw, y + bh], 10, fill=fill, outline=edge)
    for i, t in enumerate(lines):
        d.text((x + 12, y + 9 + 20 * i), t, font=f13, fill="#222222")


bubble(16, 62, ["您好，请提供订单号和注册手机号，", "我帮您查询退款进度。"])
bubble(0, 132, ["订单号 2026101908812，手机 138 0013 8000"], me=True, right=True)
bubble(16, 200, ["收到。退款已于今天 14:05 原路退回，", "预计 1–3 个工作日到账。"])
bubble(16, 268, ["如需凭证，可在 “我的 → 订单 → 退款详情” 查看。"])
bubble(0, 336, ["好的，谢谢！"], me=True, right=True)

OUT.parent.mkdir(exist_ok=True)
img.save(OUT)
print(OUT, img.size)
