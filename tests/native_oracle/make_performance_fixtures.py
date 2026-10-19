"""Deterministic synthetic benchmark inputs; never consumes a user image.

PNG creation here is test-data generation. Product transforms/exports remain Rust.
"""
from pathlib import Path
import hashlib,json
from PIL import Image,ImageDraw

ROOT=Path(__file__).resolve().parents[2]
DEST=ROOT/'poc/native-egui/assets/performance'

def make(width,height):
    raw=bytearray()
    for y in range(height):
        row=bytes(v for x in range(width) for v in (x%251,y%251,(x//251+(y//251)*101)%256))
        raw.extend(row)
    image=Image.frombytes('RGB',(width,height),bytes(raw))
    draw=ImageDraw.Draw(image)
    # Global location fiducials plus 1px lines, deliberately opaque synthetic data.
    for x,y,color in [(0,0,'red'),(width-64,0,'green'),(0,height-64,'blue'),(width-64,height-64,'yellow'),(width//2,height//2,'magenta')]:
        draw.rectangle((x,y,x+63,y+63),fill=color)
        draw.line((x,y,x+63,y+63),fill='white',width=1)
    draw.line((0,height//2,width-1,height//2),fill='black',width=1)
    return image

def main():
    DEST.mkdir(parents=True,exist_ok=True)
    files=[]
    for name,w,h in [('grid-24mp.png',6000,4000),('wide-24mp.png',24000,1000)]:
        image=make(w,h);path=DEST/name
        if path.exists():
            with Image.open(path) as old:assert old.convert('RGB').tobytes()==image.tobytes()
        else:image.save(path,format='PNG',compress_level=6)
        data=path.read_bytes();assert w*h==24_000_000 and len(data)<=25*1024*1024
        files.append({'file':name,'size':[w,h],'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),
                      'rgb_sha256':hashlib.sha256(image.tobytes()).hexdigest(),'alpha':'opaque'})
    report={'schema':1,'kind':'fixed synthetic 24MP RGB PNG with global fiducials and 1px lines','generator':'tests/native_oracle/make_performance_fixtures.py',
            'formula':'RGB=(x%251,y%251,(x//251+(y//251)*101)%256), then five 64px color/white-diagonal fiducials and a black center 1px horizontal line',
            'license':'CC0-1.0; generated project synthetic pixels, no external/private image','budget_scope':'two compressible synthetic PNGs; not worst-case entropy/JPEG/alpha/all inputs',
            'files':files}
    (DEST/'PROVENANCE.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(report))

if __name__=='__main__':main()
