"""M0 fixtures for the 0.4 privacy check: invented values only (IPs from documentation
ranges, example.com mail), one labelled line per image, plus each value's glyph box.
Writes fixtures/s-*.bgra and fixtures/sensitive.tsv."""
import os
import random
import string
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).parent
OUT = ROOT / 'fixtures'
FONTS = {
    'noto': str(ROOT.parents[1] / 'assets/fonts/NotoSansCJKsc-Regular.otf'),
    'yahei': 'C:/Windows/Fonts/msyh.ttc',
    'consolas': 'C:/Windows/Fonts/consola.ttf',
}
SIZES = [12, 13, 14, 16, 20]
THEMES = {'light': ((255, 255, 255), (34, 34, 34)), 'dark': ((30, 30, 30), (220, 220, 220))}
# HELDOUT=1: fresh values and fonts the rules were not tuned on.
HELDOUT = bool(os.environ.get('HELDOUT'))
if HELDOUT:
    FONTS = {'yahei': FONTS['yahei'], 'arial': 'C:/Windows/Fonts/arial.ttf', 'simsun': 'C:/Windows/Fonts/simsun.ttc'}
rng = random.Random(405 if HELDOUT else 404)


def digits(n):
    return ''.join(rng.choice(string.digits) for _ in range(n))


def phone():
    return '1' + rng.choice('3456789') + digits(9)


def id_card():
    body = rng.choice(['110101', '310104', '440106']) + f'{rng.randint(1970, 2005)}{rng.randint(1, 12):02d}{rng.randint(1, 28):02d}' + digits(3)
    weights = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2]
    return body + '10X98765432'[sum(int(c) * w for c, w in zip(body, weights)) % 11]


def card():
    body = '6222' + digits(14)
    total = 0
    for i, c in enumerate(reversed(body)):
        d = int(c) * (2 if i % 2 == 0 else 1)
        total += d - 9 if d > 9 else d
    return body + str((10 - total % 10) % 10)


def email():
    return ''.join(rng.choice(string.ascii_lowercase) for _ in range(rng.randint(5, 10))) + digits(2) + '@example.com'


def ipv4():
    return rng.choice(['192.0.2', '198.51.100', '203.0.113']) + f'.{rng.randint(1, 254)}'


def key():
    tail = ''.join(rng.choice(string.ascii_letters + string.digits) for _ in range(28))
    return rng.choice(['sk-', 'ghp_', 'AKIA']) + tail


KINDS = {'phone': ('联系电话：', phone), 'id': ('身份证号：', id_card), 'card': ('银行卡号：', card),
         'email': ('邮箱 ', email), 'ip': ('服务器地址 ', ipv4), 'key': ('token: ', key)}

rows = []
n = 0
for fname, path in FONTS.items():
    for size in SIZES:
        font = ImageFont.truetype(path, size)
        cjk = ImageFont.truetype(FONTS['yahei'] if fname in ('consolas', 'arial') else path, size)
        for theme, (bg, fg) in THEMES.items():
            for kind, (label, make) in KINDS.items():
                for _ in range(3):
                    value = make()
                    img = Image.new('RGB', (760, size * 3 + 16), bg)
                    d = ImageDraw.Draw(img)
                    d.fontmode = 'L'
                    x0 = 16 + d.textlength(label, font=cjk)
                    y = size // 2 + 8
                    # Consolas has no CJK glyphs: real apps fall back, so labels do too.
                    d.text((16, y), label, font=cjk, fill=fg)
                    d.text((x0, y), value, font=font, fill=fg)
                    d.text((x0 + d.textlength(value, font=font) + size, y), '备注', font=cjk, fill=fg)
                    name = f's-{n:04d}.bgra'
                    box = d.textbbox((x0, y), value, font=font)
                    w, h = img.size
                    raw = bytearray(w.to_bytes(4, 'little') + h.to_bytes(4, 'little'))
                    raw += img.convert('RGBA').tobytes('raw', 'BGRA')
                    (OUT / name).write_bytes(raw)
                    rows.append('\t'.join(map(str, [name, fname, size, theme, kind, value, *box])))
                    n += 1
# Ordinary lines that must not be flagged, to measure false positives.
ORDINARY = ['订单时间 2024-10-15 14:30:25', '价格 ¥1,299.00 含运费', '版本 1.18.1 已发布', '文件大小 35128320 字节',
            '第 123 页，共 456 页', 'commit 09b001c 修复', 'Build 19045.3803', '共 1024 条记录', '2024年10月15日 星期二',
            '比例 16:9 分辨率 1920×1080', '耗时 1173 ms 内存 562 MB', '发票号 44031234 金额 88.50']
for fname, path in FONTS.items():
    if fname in ('consolas', 'arial'):
        path = FONTS['yahei']
    for size in SIZES:
        font = ImageFont.truetype(path, size)
        for theme, (bg, fg) in THEMES.items():
            for line in ORDINARY:
                img = Image.new('RGB', (760, size * 3 + 16), bg)
                d = ImageDraw.Draw(img)
                d.fontmode = 'L'
                d.text((16, size // 2 + 8), line, font=font, fill=fg)
                name = f's-{n:04d}.bgra'
                w, h = img.size
                raw = bytearray(w.to_bytes(4, 'little') + h.to_bytes(4, 'little'))
                raw += img.convert('RGBA').tobytes('raw', 'BGRA')
                (OUT / name).write_bytes(raw)
                rows.append('	'.join(map(str, [name, fname, size, theme, 'none', line, 0, 0, 0, 0])))
                n += 1
(OUT / 'sensitive.tsv').write_text('\n'.join(rows) + '\n', encoding='utf-8')
print(n, 'images')
