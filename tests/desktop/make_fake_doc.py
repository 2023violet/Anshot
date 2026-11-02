"""A synthetic page of invented personal data for the privacy-check e2e.

Usage: python tests/desktop/make_fake_doc.py <out.png> <truth.json>
Every value is made up: IPs come from documentation ranges, mail from example.com, and the ID
and card numbers are generated to pass their check digits but belong to nobody.
"""
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

FONT = Path(__file__).parents[2] / 'assets/fonts/NotoSansCJKsc-Regular.otf'
LINES = [
    ('phone', '联系电话：', '13800138000'),
    ('id', '身份证号：', '11010519491231002X'),
    ('card', '银行卡号：', '6222020000000000000'),
    ('email', '邮箱：', 'wangxiaoming@example.com'),
    ('ip', '服务器：', '192.0.2.10'),
    ('key', '密钥：', 'ghp_2NiPnAn76ooJUsqA5qxv5r5WXXQF'),
    ('name', '客户：', '张三'),
    ('none', '备注：', '版本 1.18.1，共 1024 条记录'),
    ('name', '联系人：', '张三'),
]


def main() -> None:
    out, truth = sys.argv[1:]
    font = ImageFont.truetype(str(FONT), 20)
    img = Image.new('RGB', (900, 560), (255, 255, 255))
    d = ImageDraw.Draw(img)
    rows = []
    for i, (kind, label, value) in enumerate(LINES):
        y = 30 + i * 56
        d.text((40, y), label, font=font, fill=(34, 34, 34))
        x = 40 + d.textlength(label, font=font)
        d.text((x, y), value, font=font, fill=(34, 34, 34))
        rows.append({'kind': kind, 'value': value, 'box': list(d.textbbox((x, y), value, font=font))})
    img.save(out)
    Path(truth).write_text(json.dumps(rows, ensure_ascii=False, indent=1), encoding='utf-8')


main()
