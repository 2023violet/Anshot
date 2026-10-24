"""Check a long screenshot of the hard-case fixture (scroll_window.ps1 -Side/-Header/-Footer).

The selection started `inset` px inside the fixture window, so the stitched result must be:
`header - inset` rows of the header colour, then the page from its first row, then
`footer - inset` rows of the footer colour; only the scrolling pane's columns are kept
(the sidebar is cut away), and the page occupies the first `page width` columns.

Usage: python check_longshot.py result.png page.png HEADER FOOTER INSET
Prints LONGHARD_PASS or a reason.
"""
import sys
from PIL import Image

HEADER_RGB = (40, 90, 200)
FOOTER_RGB = (30, 140, 80)


def main(result_path, page_path, header, footer, inset):
    res = Image.open(result_path).convert('RGB')
    page = Image.open(page_path).convert('RGB')
    rw, rh = res.size
    pw, ph = page.size
    hh, ff = header - inset, footer - inset
    if rw < pw:
        return f'FAIL width {rw} < page width {pw} (the pane was cut too narrow)'
    if rw > pw + 60:
        return f'FAIL width {rw}: the still sidebar was not cut away'
    body = rh - hh - ff
    if body <= 0 or body > ph:
        return f'FAIL height {rh}: body {body} rows'
    px = res.load()
    for y in range(hh):
        for x in range(pw):
            if px[x, y] != HEADER_RGB:
                return f'FAIL header row {y} x {x} = {px[x, y]}'
    for y in range(rh - ff, rh):
        for x in range(pw):
            if px[x, y] != FOOTER_RGB:
                return f'FAIL footer row {y} x {x} = {px[x, y]}'
    got = res.crop((0, hh, pw, hh + body))
    want = page.crop((0, 0, pw, body))
    if got.tobytes() != want.tobytes():
        diff = sum(1 for a, b in zip(got.getdata(), want.getdata()) if a != b)
        return f'FAIL body differs from the page in {diff} px'
    return f'LONGHARD_PASS size={rw}x{rh} header={hh} body={body} footer={ff}'


if __name__ == '__main__':
    a = sys.argv[1:]
    print(main(a[0], a[1], int(a[2]), int(a[3]), int(a[4])))
