//! M4 intake hardening: hostile corpus + seeded byte mutations through every
//! public intake function. No new dependencies: a counting global allocator
//! measures per-call memory and a tiny xorshift64 drives the mutator.
//!
//! Default `cargo test` runs the embedded critical cases, the generated corpus
//! (skipped with a notice when `python tests/security/gen_corpus.py` was not
//! run) and a short mutation smoke run. Full run, see docs/SECURITY_TESTING.md:
//!   ANSHOT_FUZZ_ITERS=100000 cargo test --release --locked -p anshot-core --test intake_hardening
//! `ANSHOT_FUZZ_SEED=<u64>` replays a run; `ANSHOT_FUZZ_TIMEOUT_SECS` bounds one
//! call (a watchdog aborts on a hang). A failing input is written to
//! the cargo target tmpdir before the test fails.
use image::{DynamicImage,ImageFormat,Rgba,RgbaImage};
use anshot_core::{decode_generated_png,decode_source,dib_to_png,MAX_DIB_BYTES,MAX_INPUT_BYTES,MAX_PIXELS};
use std::alloc::{GlobalAlloc,Layout,System};
use std::io::Cursor;
use std::panic::{catch_unwind,AssertUnwindSafe};
use std::path::{Path,PathBuf};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool,AtomicU64,AtomicUsize,Ordering::Relaxed};
use std::sync::{Mutex,Once,OnceLock};
use std::time::{Duration,Instant};

// ------------------------------------------------------------ memory meter
struct Meter;
static LIVE:AtomicUsize=AtomicUsize::new(0);
static PEAK:AtomicUsize=AtomicUsize::new(0);
static LARGEST:AtomicUsize=AtomicUsize::new(0);
fn grow(n:usize){let now=LIVE.fetch_add(n,Relaxed)+n;PEAK.fetch_max(now,Relaxed);LARGEST.fetch_max(n,Relaxed);}
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self,l:Layout)->*mut u8{let p=System.alloc(l);if !p.is_null(){grow(l.size());}p}
    unsafe fn alloc_zeroed(&self,l:Layout)->*mut u8{let p=System.alloc_zeroed(l);if !p.is_null(){grow(l.size());}p}
    unsafe fn dealloc(&self,p:*mut u8,l:Layout){System.dealloc(p,l);LIVE.fetch_sub(l.size(),Relaxed);}
    unsafe fn realloc(&self,p:*mut u8,l:Layout,new:usize)->*mut u8{
        let q=System.realloc(p,l,new);
        if !q.is_null(){LIVE.fetch_sub(l.size(),Relaxed);grow(new);}
        q
    }
}
#[global_allocator] static METER:Meter=Meter;
/// Tests in this binary share the meter, so they run one at a time.
static SERIAL:Mutex<()>=Mutex::new(());

/// Largest legitimate single buffer: a 24 MP RGBA16 decode (MAX_PIXELS*8 B).
const LARGEST_ALLOWED:usize=MAX_PIXELS as usize*8;
/// Per-call growth ceiling: image::Limits::default().max_alloc, the same
/// 512 MiB budget decode_generated_png sets explicitly.
const PEAK_ALLOWED:usize=512*1024*1024;

#[derive(Default)]
struct Stats{calls:usize,ok:usize,peak:usize,largest:usize,
    /// Decoder panics that decode_source contained and turned into Err, by location.
    contained:BTreeMap<String,usize>}
impl Stats{
    fn report(&self,what:&str,started:Instant){
        eprintln!("{what}: {} calls, {} accepted, peak growth {} B, largest single allocation {} B, contained decoder panics {:?}, {:.1}s",
            self.calls,self.ok,self.peak,self.largest,self.contained,started.elapsed().as_secs_f64());
    }
}

// --------------------------------------------- panic and hang observation
static IN_CALL:AtomicBool=AtomicBool::new(false);
static PANICS:AtomicUsize=AtomicUsize::new(0);
static LAST_PANIC:Mutex<String>=Mutex::new(String::new());
/// Milliseconds since TIMER at which the current intake call started; 0 when idle.
static CALL_STARTED:AtomicU64=AtomicU64::new(0);
static CURRENT:Mutex<(String,Vec<u8>)>=Mutex::new((String::new(),Vec::new()));
static TIMER:OnceLock<Instant>=OnceLock::new();
fn now_ms()->u64{TIMER.get_or_init(Instant::now).elapsed().as_millis() as u64+1}
fn lock<T>(m:&Mutex<T>)->std::sync::MutexGuard<'_,T>{m.lock().unwrap_or_else(|e|e.into_inner())}
/// Panics inside an intake call are recorded quietly (the call's result is
/// judged by `check`); any other panic keeps the default report. A watchdog
/// aborts the run if one call exceeds ANSHOT_FUZZ_TIMEOUT_SECS (default 120)
/// and keeps that input, since a hang cannot be caught in-thread.
fn observe(){
    static ONCE:Once=Once::new();
    ONCE.call_once(||{
        let default=std::panic::take_hook();
        std::panic::set_hook(Box::new(move|info|{
            if IN_CALL.load(Relaxed){
                PANICS.fetch_add(1,Relaxed);
                *lock(&LAST_PANIC)=format!("{} ({})",
                    info.location().map(|l|format!("{}:{}",{let f=l.file().replace('\\',"/");let parts:Vec<&str>=f.split('/').collect();
                        // Registry paths keep "<crate>-<version>/src/...", others the last three parts.
                        let from=parts.iter().position(|p|p.starts_with("index.crates.io")).map_or(parts.len().saturating_sub(3),|i|i+1);
                        parts[from..].join("/")},l.line())).unwrap_or_default(),
                    info.payload().downcast_ref::<String>().cloned().or_else(||info.payload().downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or_default());
            }else{default(info);}
        }));
        let limit=std::env::var("ANSHOT_FUZZ_TIMEOUT_SECS").ok().and_then(|v|v.parse().ok()).unwrap_or(120u64)*1000;
        std::thread::spawn(move||loop{
            std::thread::sleep(Duration::from_millis(500));
            let started=CALL_STARTED.load(Relaxed);
            if started!=0 && now_ms().saturating_sub(started)>limit{
                let (label,input)=&*lock(&CURRENT);
                eprintln!("HANG: intake call {label} exceeded {} s; input kept at {}",limit/1000,keep_failure(&format!("hang-{label}"),input).display());
                std::process::abort();
            }
        });
    });
}

#[derive(Clone,Copy,Debug)]
enum Target{Source,Dib,Generated(u32,u32)}

/// Run one intake call; Err(message) on escaped panic, limit breach or invalid Ok.
fn check(label:&str,target:Target,input:&[u8],stats:&mut Stats)->Result<(),String>{
    observe();
    {let mut current=lock(&CURRENT);current.0.clear();current.0.push_str(label);current.1.clear();current.1.extend_from_slice(input);}
    let panics_before=PANICS.load(Relaxed);
    let base=LIVE.load(Relaxed);PEAK.store(base,Relaxed);LARGEST.store(0,Relaxed);
    IN_CALL.store(true,Relaxed);CALL_STARTED.store(now_ms(),Relaxed);
    let outcome=catch_unwind(AssertUnwindSafe(||->Result<bool,String>{
        Ok(match target{
            Target::Source=>match decode_source(input){
                Ok(img)=>{valid_image(&img)?;true},Err(_)=>false},
            Target::Dib=>match dib_to_png(input){
                Ok(png)=>{
                    if png.len()>MAX_INPUT_BYTES{return Err(format!("DIB PNG {} B over source limit",png.len()));}
                    let img=decode_source(&png).map_err(|e|format!("DIB PNG not re-decodable: {e}"))?;
                    valid_image(&img)?;
                    let (w,h)=(i32::from_le_bytes(input[4..8].try_into().unwrap()),i32::from_le_bytes(input[8..12].try_into().unwrap()));
                    if img.dimensions()!=(w as u32,h.unsigned_abs()){return Err("DIB PNG changed dimensions".into());}
                    true
                },Err(_)=>false},
            Target::Generated(w,h)=>match decode_generated_png(input,(w,h)){
                Ok(img)=>{if img.dimensions()!=(w,h){return Err("generated readback changed dimensions".into());}true},
                Err(_)=>false},
        })
    }));
    CALL_STARTED.store(0,Relaxed);IN_CALL.store(false,Relaxed);
    let peak=PEAK.load(Relaxed).saturating_sub(base);let largest=LARGEST.load(Relaxed);
    stats.calls+=1;stats.peak=stats.peak.max(peak);stats.largest=stats.largest.max(largest);
    let panicked=PANICS.load(Relaxed)!=panics_before;
    match outcome{
        Err(_)=>Err(format!("panic escaped intake: {}",lock(&LAST_PANIC))),
        Ok(Err(invalid))=>Err(invalid),
        Ok(Ok(ok))=>{
            if panicked{
                if ok{return Err(format!("accepted after an internal panic: {}",lock(&LAST_PANIC)));}
                let at=lock(&LAST_PANIC).split(" (").next().unwrap_or_default().to_string();
                *stats.contained.entry(at).or_default()+=1;
            }
            stats.ok+=usize::from(ok);
            if largest>LARGEST_ALLOWED{return Err(format!("single allocation {largest} B > {LARGEST_ALLOWED}"));}
            if peak>PEAK_ALLOWED{return Err(format!("peak growth {peak} B > {PEAK_ALLOWED}"));}
            Ok(())
        }
    }
}
fn valid_image(img:&RgbaImage)->Result<(),String>{
    let (w,h)=img.dimensions();
    if w==0 || h==0 || u64::from(w)*u64::from(h)>MAX_PIXELS || img.as_raw().len()!=w as usize*h as usize*4{
        return Err(format!("Ok image outside limits: {w}x{h}"));
    }
    Ok(())
}
fn keep_failure(label:&str,input:&[u8])->PathBuf{
    let path=Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("intake-failure-{label}.bin"));
    let _=std::fs::write(&path,input);path
}
fn must_hold(label:&str,target:Target,input:&[u8],stats:&mut Stats){
    if let Err(why)=check(label,target,input,stats){
        panic!("{label} ({target:?}): {why}; input kept at {}",keep_failure(label,input).display());
    }
}

// ------------------------------------------------------------- byte makers
fn crc32(bytes:&[u8])->u32{
    let mut crc=u32::MAX;
    for &b in bytes{crc^=u32::from(b);for _ in 0..8{crc=if crc&1!=0{(crc>>1)^0xedb8_8320}else{crc>>1};}}
    !crc
}
fn chunk(kind:&[u8;4],data:&[u8])->Vec<u8>{
    let mut out=(data.len() as u32).to_be_bytes().to_vec();out.extend(kind);out.extend(data);
    out.extend(crc32(&out[4..]).to_be_bytes());out
}
fn ihdr(w:u32,h:u32)->Vec<u8>{
    let mut d=w.to_be_bytes().to_vec();d.extend(h.to_be_bytes());d.extend([8,6,0,0,0]);chunk(b"IHDR",&d)
}
/// IDAT holding a stored (uncompressed) zlib stream of `len` zero bytes.
fn stored_idat(len:usize)->Vec<u8>{
    let mut z=vec![0x78,0x01];
    for (i,block) in vec![0u8;len].chunks(65535).enumerate(){
        let last=(i+1)*65535>=len;
        z.push(u8::from(last));z.extend((block.len() as u16).to_le_bytes());z.extend((!(block.len() as u16)).to_le_bytes());
        z.extend(block);
    }
    z.extend(1u32.to_be_bytes());// wrong Adler on purpose: decoders must still stop cleanly
    chunk(b"IDAT",&z)
}
const MAGIC:&[u8]=b"\x89PNG\r\n\x1a\n";
fn png_with(parts:&[Vec<u8>])->Vec<u8>{
    let mut out=MAGIC.to_vec();for p in parts{out.extend(p);}out.extend(chunk(b"IEND",b""));out
}
fn encoded(image:DynamicImage,format:ImageFormat)->Vec<u8>{
    let mut out=Cursor::new(Vec::new());image.write_to(&mut out,format).unwrap();out.into_inner()
}
fn sample(w:u32,h:u32)->RgbaImage{
    RgbaImage::from_fn(w,h,|x,y|Rgba([(x*29) as u8,(y*53) as u8,(x^y) as u8,255]))
}
fn jpeg_with_app1(jpeg:&[u8],payload:&[u8])->Vec<u8>{
    let mut out=jpeg[..2].to_vec();out.extend([0xff,0xe1]);out.extend(((payload.len()+2) as u16).to_be_bytes());
    out.extend(payload);out.extend(&jpeg[2..]);out
}
fn exif_orientation(value:u16,kind:u16,count:u32)->Vec<u8>{
    let mut t=b"Exif\0\0II".to_vec();t.extend(42u16.to_le_bytes());t.extend(8u32.to_le_bytes());
    t.extend(1u16.to_le_bytes());t.extend(0x0112u16.to_le_bytes());t.extend(kind.to_le_bytes());
    t.extend(count.to_le_bytes());t.extend(value.to_le_bytes());t.extend([0;2]);t.extend(0u32.to_le_bytes());t
}
fn dib(header:u32,w:i32,h:i32,bits:u16,compression:u32,pixels:usize)->Vec<u8>{
    let mut b=vec![0;header.max(40) as usize];
    b[0..4].copy_from_slice(&header.to_le_bytes());b[4..8].copy_from_slice(&w.to_le_bytes());
    b[8..12].copy_from_slice(&h.to_le_bytes());b[12]=1;b[14..16].copy_from_slice(&bits.to_le_bytes());
    b[16..20].copy_from_slice(&compression.to_le_bytes());
    if header>=108{
        for (at,v) in [(40,0x00ff0000u32),(44,0x0000ff00),(48,0x000000ff),(52,0xff000000),(56,0x73524742)]{
            b[at..at+4].copy_from_slice(&v.to_le_bytes());
        }
    }
    b.extend((0..pixels).map(|i|(i*37) as u8));b
}

// --------------------------------------------------------- critical cases
#[test]
fn embedded_critical_cases_fail_closed_within_limits(){
    let _serial=lock(&SERIAL);let started=Instant::now();
    let mut stats=Stats::default();
    let png=encoded(DynamicImage::ImageRgba8(sample(8,6)),ImageFormat::Png);
    let jpeg=encoded(DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(sample(8,6)).to_rgb8()),ImageFormat::Jpeg);
    let rejected:Vec<(&str,Vec<u8>)>=vec![
        ("empty",vec![]),
        ("over_25mib",{let mut b=png.clone();b.resize(MAX_INPUT_BYTES+1,0);b}),
        ("bomb_100000_sq",png_with(&[ihdr(100_000,100_000),stored_idat(64)])),
        ("bomb_u32max",png_with(&[ihdr(u32::MAX,u32::MAX),stored_idat(1)])),
        ("over_24mp",png_with(&[ihdr(24_000_001,1),stored_idat(1)])),
        ("zero_width",png_with(&[ihdr(0,5),stored_idat(1)])),
        ("tiny_idat_at_24mp",png_with(&[ihdr(4898,4898),stored_idat(64)])),
        ("chunk_before_ihdr",{let mut b=MAGIC.to_vec();b.extend(chunk(b"tEXt",b"k\0v"));b.extend(&png[8..]);b}),
        ("bad_ihdr_crc",{let mut b=png.clone();b[29]^=0xff;b}),
        ("truncated_png",png[..png.len()/2].to_vec()),
        ("truncated_jpeg",jpeg[..jpeg.len()/2].to_vec()),
        ("soi_only",jpeg[..3].to_vec()),
    ];
    for (label,bytes) in &rejected{
        must_hold(label,Target::Source,bytes,&mut stats);
        assert!(decode_source(bytes).is_err(),"{label} must be rejected");
    }
    // Deflate bomb: 1x1 image whose image data inflates to 32 MiB.
    let bomb=png_with(&[{let mut d=1u32.to_be_bytes().to_vec();d.extend(1u32.to_be_bytes());d.extend([8,6,0,0,0]);chunk(b"IHDR",&d)},stored_idat(32<<20)]);
    must_hold("deflate_bomb_1x1",Target::Source,&bomb,&mut stats);
    // Orientation values outside 1..=8, the wrong TIFF type and count>1 are ignored, never trusted.
    for (value,kind,count) in [(0,3,1),(9,3,1),(255,3,1),(u16::MAX,3,1),(6,4,1),(6,3,2)]{
        let bytes=jpeg_with_app1(&jpeg,&exif_orientation(value,kind,count));
        must_hold("exif_orientation",Target::Source,&bytes,&mut stats);
        assert_eq!(decode_source(&bytes).unwrap().dimensions(),(8,6),"value={value} kind={kind} count={count}");
    }
    let rotated=jpeg_with_app1(&jpeg,&exif_orientation(6,3,1));
    assert_eq!(decode_source(&rotated).unwrap().dimensions(),(6,8));
    // Clipboard DIB headers with absurd sizes, heights and bit depths.
    for (label,bytes) in [
        ("w_i32max",dib(40,i32::MAX,1,24,0,4096)),("h_i32min",dib(40,4,i32::MIN,24,0,4096)),
        ("w_negative",dib(40,-4,4,24,0,4096)),("65536_sq",dib(40,65536,65536,32,0,4096)),
        ("24mp_plus1",dib(40,4899,-4899,32,0,4096)),("claims_24mp_no_pixels",dib(40,6000,4000,32,0,16)),
        ("bits_8",dib(40,4,4,8,0,4096)),("bits_16",dib(40,4,4,16,0,4096)),("bits_64",dib(40,4,4,64,0,4096)),
        ("rle8",dib(40,4,4,8,1,4096)),("bitfields_24",dib(40,4,4,24,3,4096)),
        ("header_12",{let mut b=dib(40,4,4,24,0,64);b[0]=12;b}),("header_u32max",{let mut b=dib(40,4,4,24,0,64);b[0..4].fill(0xff);b}),
        ("truncated_row",dib(40,7,-5,24,0,7*3+1+24*3)),("over_dib_limit",vec![40;MAX_DIB_BYTES+1]),
    ]{
        must_hold(label,Target::Dib,&bytes,&mut stats);
        assert!(dib_to_png(&bytes).is_err(),"DIB {label} must be rejected");
    }
    let v5=dib(124,2,-2,32,3,16);
    must_hold("v5_ok",Target::Dib,&v5,&mut stats);
    assert!(dib_to_png(&v5).is_ok());
    // Generated readback: a deflate bomb with the right header must still fail closed.
    let gen_bomb=png_with(&[ihdr(5,3),stored_idat(16<<20)]);
    must_hold("generated_bomb",Target::Generated(5,3),&gen_bomb,&mut stats);
    stats.report("embedded",started);
}

// ------------------------------------------------------- generated corpus
fn corpus_dir()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/security/corpus")}

#[test]
fn generated_corpus_is_handled_within_limits(){
    let _serial=lock(&SERIAL);let started=Instant::now();
    let Ok(entries)=std::fs::read_dir(corpus_dir()) else{
        eprintln!("SKIPPED: no corpus at {}; run `python tests/security/gen_corpus.py`",corpus_dir().display());
        return;
    };
    let mut names:Vec<_>=entries.filter_map(|e|e.ok()).map(|e|e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    let mut stats=Stats::default();let mut failures=Vec::new();
    for name in &names{
        let bytes=std::fs::read(corpus_dir().join(name)).unwrap();
        let target=if name.starts_with("src_"){Target::Source}
            else if name.starts_with("dib_"){Target::Dib}
            else if let Some(size)=name.strip_prefix("gen_").and_then(|n|n.split('_').next()){
                let (w,h)=size.split_once('x').unwrap();Target::Generated(w.parse().unwrap(),h.parse().unwrap())
            }else{continue};
        let before=stats.ok;
        if let Err(why)=check(name,target,&bytes,&mut stats){failures.push(format!("{name}: {why}"));}
        let accepted=stats.ok>before;
        // Spot-check expectations that encode real product policy.
        let must_reject=["bomb","over_25mib","over24mp","24mp_plus1","empty","wrong_magic","truncated","bad_crc",
            "chunk_before_ihdr","idat_before_ihdr","dims_","bits_","planes_","compression_","header_size_","_cut_"];
        // Lenient by design: malformed optional EXIF is ignored (orientation 1), zune-jpeg
        // decodes a missing EOI or short scan as filled pixels, a deflate bomb stops at the
        // image size, and a 4000x6000 SOF sits exactly at the 24 MP gate.
        let lenient=["exif_","deflate_bomb","edge24mp"];
        if must_reject.iter().any(|k|name.contains(k)) && !lenient.iter().any(|k|name.contains(k))
            && accepted && !name.contains("valid"){
            failures.push(format!("{name}: expected rejection but intake accepted it"));
        }
        let must_accept=["valid","exif_orient_","jpg_cmyk","jpg_grayscale","jpg_progressive.jpg","jpg_subsampling",
            "png_apng","png_16bit","png_1bit","png_palette.png","png_interlaced_rgba","png_interlaced_16bit","png_zeros_4000"];
        if must_accept.iter().any(|k|name.contains(k)) && !name.contains("invalid") && !accepted{
            failures.push(format!("{name}: well-formed input was rejected"));
        }
    }
    stats.report("corpus",started);
    assert!(failures.is_empty(),"corpus failures:\n{}",failures.join("\n"));
}

// --------------------------------------------------------------- mutator
struct XorShift(u64);
impl XorShift{
    /// splitmix64 spreads user seeds so 2 and 3 give different streams.
    fn seeded(seed:u64)->Self{
        let mut z=seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z=(z^(z>>30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);z=(z^(z>>27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        Self((z^(z>>31)).max(1))
    }
    fn next(&mut self)->u64{let mut x=self.0;x^=x<<13;x^=x>>7;x^=x<<17;self.0=x;x}
    fn below(&mut self,n:usize)->usize{(self.next()%n.max(1) as u64) as usize}
}
const INTERESTING32:[u32;10]=[0,1,0x7f,0xff,0xffff,0x7fff_ffff,0x8000_0000,u32::MAX,24_000_001,65536];
fn mutate(rng:&mut XorShift,seed:&[u8])->Vec<u8>{
    let mut b=seed.to_vec();
    for _ in 0..1+rng.below(4){
        let len=b.len();
        match rng.below(9){
            0 if len>0=>{let at=rng.below(len);b[at]^=1<<rng.below(8);},
            1 if len>0=>{let at=rng.below(len);b[at]=rng.next() as u8;},
            2 if len>0=>{let at=rng.below(len);b[at]=[0,0xff,0x7f,0x80][rng.below(4)];},
            3 if len>=4=>{let at=rng.below(len-3);let v=INTERESTING32[rng.below(INTERESTING32.len())];
                b[at..at+4].copy_from_slice(&if rng.below(2)==0{v.to_be_bytes()}else{v.to_le_bytes()});},
            4 if len>0=>{b.truncate(rng.below(len));},
            5 if len>0=>{let at=rng.below(len);let n=1+rng.below(16.min(len-at));b.drain(at..at+n);},
            6=>{let at=rng.below(len+1);let n=1+rng.below(32);let fill=rng.next() as u8;b.splice(at..at,std::iter::repeat_n(fill,n));},
            7 if len>8=>{let from=rng.below(len-8);let n=1+rng.below(64.min(len-from));let piece=b[from..from+n].to_vec();
                let at=rng.below(len);b.splice(at..at,piece);},
            _ if len>0=>{let at=rng.below(len);b[at]=b[at].wrapping_add(1+rng.below(4) as u8);},
            _=>b.push(rng.next() as u8),
        }
    }
    b
}

fn mutation_seeds()->Vec<(&'static str,Target,Vec<u8>)>{
    let rgba=sample(8,6);
    let rgb=DynamicImage::ImageRgba8(rgba.clone()).to_rgb8();
    let gray16=image::ImageBuffer::<image::Luma<u16>,_>::from_fn(5,4,|x,y|image::Luma([(x*9000+y*700) as u16]));
    let generated=encoded(DynamicImage::ImageRgba8(sample(5,3)),ImageFormat::Png);
    let mut seeds=vec![
        ("png_rgba8",Target::Source,encoded(DynamicImage::ImageRgba8(rgba),ImageFormat::Png)),
        ("png_rgb8",Target::Source,encoded(DynamicImage::ImageRgb8(rgb.clone()),ImageFormat::Png)),
        ("png_l16",Target::Source,encoded(DynamicImage::ImageLuma16(gray16),ImageFormat::Png)),
        ("jpeg_rgb",Target::Source,encoded(DynamicImage::ImageRgb8(rgb.clone()),ImageFormat::Jpeg)),
        ("jpeg_exif6",Target::Source,jpeg_with_app1(&encoded(DynamicImage::ImageRgb8(rgb),ImageFormat::Jpeg),&exif_orientation(6,3,1))),
        ("dib24",Target::Dib,dib(40,3,-2,24,0,24)),
        ("dib32_v5",Target::Dib,dib(124,3,2,32,3,24)),
        ("dib32_bitfields40",Target::Dib,{let mut b=dib(40,2,2,32,3,0);
            b.extend([0x00ff0000u32,0x0000ff00,0x000000ff].into_iter().flat_map(u32::to_le_bytes));b.extend([7;16]);b}),
        ("generated",Target::Generated(5,3),generated),
    ];
    // Unusual-but-valid corpus files widen the seed set when the corpus exists.
    for name in ["src_png_palette.png","src_png_interlaced_rgba.png","src_png_apng_two_frames.png",
        "src_jpg_progressive.jpg","src_jpg_cmyk.jpg","src_png_16bit_rgba.png"]{
        if let Ok(bytes)=std::fs::read(corpus_dir().join(name)){seeds.push((name,Target::Source,bytes));}
    }
    seeds
}

#[test]
fn seeded_mutations_never_panic_or_exceed_limits(){
    let _serial=lock(&SERIAL);
    let iterations:usize=std::env::var("ANSHOT_FUZZ_ITERS").ok().and_then(|v|v.parse().ok()).unwrap_or(20_000);
    let seed:u64=std::env::var("ANSHOT_FUZZ_SEED").ok().and_then(|v|v.parse().ok()).unwrap_or(0x5eed_2026_1021);
    let seeds=mutation_seeds();
    let mut rng=XorShift::seeded(seed);
    let mut stats=Stats::default();
    let started=std::time::Instant::now();
    for i in 0..iterations{
        let (name,target,base)=&seeds[i%seeds.len()];
        let input=mutate(&mut rng,base);
        if let Err(why)=check(&format!("{seed}-{i}"),*target,&input,&mut stats){
            let kept=keep_failure(&format!("{seed}-{i}"),&input);
            panic!("iteration {i} (seed {seed}, from {name}, {target:?}): {why}; input kept at {}",kept.display());
        }
    }
    stats.report(&format!("mutations ({} seeds, seed {seed})",seeds.len()),started);
}

/// Regression inputs found by the mutation run, kept as fixtures:
/// - worker-slice-oob: zune-jpeg 0.4.21 src/worker.rs:233 indexes a 16-entry buffer
///   with a 32-wide MCU row (release and debug). Before decode_source contained
///   decoder panics this unwound into the caller.
/// - progressive-bits-left: zune-jpeg's `debug_assert!(self.bits_left >= n)` in
///   src/bitstream.rs:407 (debug builds only); its stream has no reachable EOI, so
///   the marker walk now rejects it before decoding.
/// Both must now fail closed with Err and never unwind.
/// - progressive-eof-hang: see the end of this test.
#[test]
fn zune_jpeg_panic_inputs_fail_closed(){
    let _serial=lock(&SERIAL);let started=Instant::now();
    let mut stats=Stats::default();
    for (label,bytes) in [("worker-slice-oob",&include_bytes!("fixtures/zune-jpeg-worker-slice-oob.jpg")[..]),
        ("progressive-bits-left",&include_bytes!("fixtures/zune-jpeg-progressive-bits-left.jpg")[..])]{
        must_hold(label,Target::Source,bytes,&mut stats);
        assert!(decode_source(bytes).is_err(),"{label} must be rejected");
    }
    // The slice panic needs a structurally complete JPEG, so it still reaches the
    // decoder and exercises containment; bits-left is now stopped by the marker walk.
    assert_eq!(stats.contained.get("zune-jpeg-0.4.21/src/worker.rs:233"),Some(&1),"{:?}",stats.contained);
    // Hang: progressive JPEG whose data ends before EOI. Before the marker walk in
    // decode_source this call never returned (zune-jpeg 0.4.21 src/mcu_prog.rs 'eoi loop).
    let hang=include_bytes!("fixtures/zune-jpeg-progressive-eof-hang.jpg");
    let begun=Instant::now();
    must_hold("progressive-eof-hang",Target::Source,hang,&mut stats);
    assert!(decode_source(hang).is_err());
    assert!(begun.elapsed()<Duration::from_secs(5),"took {:?}",begun.elapsed());
    stats.report("regressions",started);
}
