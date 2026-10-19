//! Bounded local file input. Pixel decoding stays in Rust Core.
use std::{io::Read,path::Path};
use shareready_core::MAX_INPUT_BYTES;

fn read_limited(reader:&mut impl Read,limit:usize)->Result<Vec<u8>,String>{
    let mut bytes=Vec::new();
    reader.take(limit as u64+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len()>limit{return Err(format!("Input exceeds {limit} byte limit"));}
    Ok(bytes)
}

pub fn read_source(path:&Path)->Result<Vec<u8>,String>{
    let mut file=std::fs::File::open(path).map_err(|e|e.to_string())?;
    if file.metadata().map_err(|e|e.to_string())?.len()>MAX_INPUT_BYTES as u64 {
        return Err(format!("Input exceeds {MAX_INPUT_BYTES} byte limit"));
    }
    // Recheck while reading: the file can grow after the metadata query.
    read_limited(&mut file,MAX_INPUT_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self,Cursor,Read};
    #[test] fn oversized_stream_stops_after_one_detection_byte(){
        let mut source=Cursor::new(vec![7_u8;1000]);
        assert!(read_limited(&mut source,16).is_err());
        assert_eq!(source.position(),17);
    }
    #[test] fn exact_limit_and_short_reads_preserve_all_input_bytes(){
        struct Short(Cursor<Vec<u8>>);
        impl Read for Short {fn read(&mut self,buf:&mut [u8])->io::Result<usize>{let n=buf.len().min(2);self.0.read(&mut buf[..n])}}
        let bytes=(0..16).collect::<Vec<u8>>();
        assert_eq!(read_limited(&mut Short(Cursor::new(bytes.clone())),16).unwrap(),bytes);
        assert_eq!(read_limited(&mut Cursor::new(Vec::new()),16).unwrap(),Vec::<u8>::new());
    }
    #[test] fn read_failure_never_returns_partial_candidate(){
        struct Fails(bool);
        impl Read for Fails {fn read(&mut self,buf:&mut [u8])->io::Result<usize>{
            if self.0{return Err(io::Error::other("synthetic read failure"));}
            self.0=true;buf[0]=42;Ok(1)
        }}
        assert!(read_limited(&mut Fails(false),16).unwrap_err().contains("synthetic read failure"));
    }
}
