"""Fixed synthetic accepted JPEG whose Rust RGBA PNG exceeds the source cap."""
from pathlib import Path
import hashlib,io,json,random,struct,sys,zlib
from PIL import Image,__version__ as pillow_version
from PIL.PngImagePlugin import PngInfo

ROOT=Path(__file__).resolve().parents[2]
EXPECTED='b190d5da92519c12034032004c2c033cbda8296a7ebfd23dd298002dcefecd40'

def main():
    raw=random.Random(0x535252).randbytes(3000*3000*3)
    encoded=io.BytesIO()
    Image.frombytes('RGB',(3000,3000),raw).save(encoded,format='JPEG',quality=90,subsampling=2)
    data=encoded.getvalue();digest=hashlib.sha256(data).hexdigest()
    if digest!=EXPECTED:raise RuntimeError('fixture encoder differs; preserve frozen fixture, do not silently replace')
    out=ROOT/'tests/performance/fixtures';out.mkdir(exist_ok=True)
    file=out/'synthetic-noise-9mp.jpg'
    if file.exists():
        if file.read_bytes()!=data:raise RuntimeError('existing fixture differs; refusing overwrite')
    else:
        with file.open('xb') as stream:stream.write(data)
    provenance={'license':'MIT','origin':'project-generated synthetic pseudorandom RGB pixels; no user image',
                'size':[3000,3000],'seed':0x535252,'generator':'Python random.Random(seed).randbytes(width*height*3)',
                'python':sys.version.split()[0],'pillow':pillow_version,'jpeg_quality':90,'jpeg_subsampling':2,
                'bytes':len(data),'sha256':digest,'scope':'9MP generated-artifact readback counterexample; not all high-entropy24MP coverage'}
    metadata=out/'PROVENANCE.json';blob=(json.dumps(provenance,indent=2)+'\n').encode()
    if metadata.exists():
        if metadata.read_bytes()!=blob:raise RuntimeError('existing provenance differs; refusing overwrite')
    else:
        with metadata.open('xb') as stream:stream.write(blob)
    animation=io.BytesIO()
    Image.new('RGBA',(2,3),(1,2,3,255)).save(animation,format='PNG',save_all=True,
        append_images=[Image.new('RGBA',(2,3),(200,100,50,255))],duration=100,loop=0)
    apng=animation.getvalue();apng_path=out/'synthetic-two-frame.png'
    if apng_path.exists():
        if apng_path.read_bytes()!=apng:raise RuntimeError('existing APNG fixture differs; refusing overwrite')
    else:
        with apng_path.open('xb') as stream:stream.write(apng)
    apng_meta=(json.dumps({'license':'MIT','origin':'project synthetic RGBA8 two-frame PNG',
        'size':[2,3],'frames':2,'rgba':[[1,2,3,255],[200,100,50,255]],'duration_ms':100,'loop':0,
        'pillow':pillow_version,'bytes':len(apng),'sha256':hashlib.sha256(apng).hexdigest()},indent=2)+'\n').encode()
    apng_metadata=out/'APNG_PROVENANCE.json'
    if apng_metadata.exists():
        if apng_metadata.read_bytes()!=apng_meta:raise RuntimeError('existing APNG provenance differs')
    else:
        with apng_metadata.open('xb') as stream:stream.write(apng_meta)
    metadata_image=io.BytesIO();info=PngInfo();info.add_text('fixture','synthetic-public')
    Image.new('RGBA',(2,3),(1,2,3,255)).save(metadata_image,format='PNG',pnginfo=info)
    plain=io.BytesIO();Image.new('RGBA',(2,3),(1,2,3,255)).save(plain,format='PNG')
    raw_png=plain.getvalue();split=bytearray(raw_png[:8]);at=8
    def chunk(kind,body):return struct.pack('>I',len(body))+kind+body+struct.pack('>I',zlib.crc32(kind+body))
    while at<len(raw_png):
        size=struct.unpack_from('>I',raw_png,at)[0];kind=raw_png[at+4:at+8];body=raw_png[at+8:at+8+size]
        if kind==b'IDAT':
            split.extend(chunk(kind,body[:len(body)//2]));split.extend(chunk(kind,body[len(body)//2:]))
        else:split.extend(raw_png[at:at+size+12])
        at+=size+12
    structures={'synthetic-metadata.png':metadata_image.getvalue(),'synthetic-split-idat.png':bytes(split)}
    for name,data in structures.items():
        path=out/name
        if path.exists():
            if path.read_bytes()!=data:raise RuntimeError('existing structure fixture differs')
        else:
            with path.open('xb') as stream:stream.write(data)
    details={'license':'MIT','origin':'project synthetic RGBA8(1,2,3,255)2x3 PNGs',
             'pillow':pillow_version,'metadata':'tEXt fixture=synthetic-public',
             'split_rule':'split IDAT body in half into two consecutive IDATs, independent stdlib CRC32',
             'files':{name:{'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()} for name,data in structures.items()}}
    structure_meta=out/'STRUCTURE_PROVENANCE.json';blob=(json.dumps(details,indent=2)+'\n').encode()
    if structure_meta.exists():
        if structure_meta.read_bytes()!=blob:raise RuntimeError('existing structure provenance differs')
    else:
        with structure_meta.open('xb') as stream:stream.write(blob)
    print(json.dumps(provenance))

if __name__=='__main__':main()
