use std::{fs::{File,OpenOptions},io::{self,Write},os::windows::{ffi::OsStrExt,fs::OpenOptionsExt,io::AsRawHandle},path::{Component,Path,Prefix}};
use windows_sys::Win32::{Foundation::GENERIC_WRITE,Storage::FileSystem::{
    DELETE,FILE_FLAG_DELETE_ON_CLOSE,FILE_DISPOSITION_INFO_EX,FILE_DISPOSITION_FLAG_ON_CLOSE,
    FileDispositionInfoEx,SetFileInformationByHandle,GetFileType,FILE_TYPE_DISK,
}};

fn validate_path(path:&Path)->io::Result<()>{
    let invalid=||io::Error::new(io::ErrorKind::InvalidInput,"Choose a regular file path; device names and alternate data streams are not supported");
    for part in path.components(){
        match part {
            Component::Prefix(prefix) if matches!(prefix.kind(),Prefix::DeviceNS(_)|Prefix::Verbatim(_))=>return Err(invalid()),
            Component::Normal(name)=>{
                if name.encode_wide().any(|c|c==u16::from(b':')){return Err(invalid());}
                let name=name.to_string_lossy();
                let stem=name.split('.').next().unwrap_or("").trim_end_matches(' ').to_uppercase();
                let port=stem.strip_prefix("COM").or_else(||stem.strip_prefix("LPT"));
                if matches!(stem.as_str(),"CON"|"PRN"|"AUX"|"NUL"|"CONIN$"|"CONOUT$") ||
                    port.is_some_and(|n|["1","2","3","4","5","6","7","8","9","¹","²","³"].contains(&n)) {return Err(invalid());}
            },_=>{}
        }
    }
    Ok(())
}

fn open_rollback_file(path:&Path)->io::Result<File>{
    validate_path(path)?;
    let file=OpenOptions::new().write(true).create_new(true).access_mode(GENERIC_WRITE|DELETE)
        .share_mode(0).custom_flags(FILE_FLAG_DELETE_ON_CLOSE).open(path)?;
    // SAFETY: this synchronous classification borrows the live handle owned by File.
    if unsafe{GetFileType(file.as_raw_handle())}!=FILE_TYPE_DISK {
        return Err(io::Error::new(io::ErrorKind::Unsupported,"Save requires a disk file"));
    }
    Ok(file)
}

fn keep_file(file:&File)->io::Result<()>{
    // ON_CLOSE without DELETE clears this handle's delete-on-close state.
    let info=FILE_DISPOSITION_INFO_EX{Flags:FILE_DISPOSITION_FLAG_ON_CLOSE};
    // SAFETY: File owns a live OS handle for the duration of this synchronous call.
    // The generated repr(C) structure and its exact size match FileDispositionInfoEx;
    // its stack storage stays alive and is not retained by the Windows API.
    let ok=unsafe {SetFileInformationByHandle(file.as_raw_handle(),FileDispositionInfoEx,
        (&info as *const FILE_DISPOSITION_INFO_EX).cast(),std::mem::size_of_val(&info) as u32)};
    if ok==0 {Err(io::Error::last_os_error())}else{Ok(())}
}

fn save_new_with(path:&Path,write:impl FnOnce(&mut File)->io::Result<()>,commit:impl FnOnce(&File)->io::Result<()>)->io::Result<()>{
    let mut file=open_rollback_file(path)?;
    write(&mut file)?;
    commit(&file)?;
    drop(file); // Release the exclusive lock before reporting success.
    Ok(())
}

/// Create a new output without overwriting any existing path. On error the OS
/// removes only this newly created file by handle identity, including on exit.
/// An unsupported on-close commit operation fails without a fallback write.
pub fn save_new(path:&Path,bytes:&[u8])->io::Result<()>{
    save_new_with(path,|file|{file.write_all(bytes)?;file.sync_all()},keep_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs,io::{self,Write},path::PathBuf};
    fn new_path(label:&str)->PathBuf {
        let dir=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/save-tests")
            .join(format!("{}-{}-{label}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&dir).unwrap();dir.join("output.png")
    }
    #[test] fn committed_file_is_complete_and_readable_after_handle_closes(){
        let path=new_path("commit");let bytes=b"complete synthetic output";
        save_new(&path,bytes).unwrap();assert_eq!(fs::read(path).unwrap(),bytes);
    }
    #[test] fn existing_file_is_never_changed(){
        let path=new_path("existing");fs::write(&path,b"existing synthetic file").unwrap();
        assert!(save_new(&path,b"replacement").is_err());
        assert_eq!(fs::read(path).unwrap(),b"existing synthetic file");
    }
    #[test] fn alternate_data_stream_cannot_modify_an_existing_file(){
        let path=new_path("ads");fs::write(&path,b"existing synthetic base file").unwrap();
        let stream=path.with_file_name("output.png:unexpected");
        let error=save_new(&stream,b"synthetic stream").unwrap_err();
        assert_eq!(error.kind(),io::ErrorKind::InvalidInput);
        assert_eq!(fs::read(path).unwrap(),b"existing synthetic base file");
    }
    #[test] fn namespace_validation_rejects_devices_without_opening_them(){
        for name in [r"\\.\NUL",r"\\?\GLOBALROOT\Device\Harddisk0\new.png","NUL","con.png","COM1.txt","LPT³","CONIN$","CONOUT$","NUL .png",r"folder:stream\new.png"] {
            assert_eq!(validate_path(Path::new(name)).unwrap_err().kind(),io::ErrorKind::InvalidInput,"{name}");
        }
        for name in [r"D:\普通目录\new.png",r"\\?\D:\long\new.png",r"\\server\share\new.png","normal.png"] {
            assert!(validate_path(Path::new(name)).is_ok(),"{name}");
        }
    }
    #[test] fn partial_write_failure_leaves_no_file_and_same_path_can_retry(){
        let path=new_path("partial");
        let result=save_new_with(&path,|f|{f.write_all(b"partial")?;Err(io::Error::new(io::ErrorKind::WriteZero,"synthetic short write"))},keep_file);
        assert!(result.is_err());assert!(!path.exists());
        save_new(&path,b"complete retry").unwrap();assert_eq!(fs::read(path).unwrap(),b"complete retry");
    }
    #[test] fn sync_and_commit_failures_leave_no_file(){
        for label in ["sync","commit"] {
            let path=new_path(label);
            let result=if label=="sync" {
                save_new_with(&path,|f|{f.write_all(b"complete but uncommitted")?;Err(io::Error::other("synthetic sync failure"))},keep_file)
            }else{
                save_new_with(&path,|f|{f.write_all(b"complete but uncommitted")?;f.sync_all()},|_|Err(io::Error::other("synthetic commit failure")))
            };
            assert!(result.is_err());assert!(!path.exists(),"{label}");
        }
    }
    #[test] fn uncommitted_handle_excludes_other_read_write_and_rename(){
        let path=new_path("exclusive");let renamed=path.with_file_name("moved.png");
        let result=save_new_with(&path,|f|{
            f.write_all(b"uncommitted")?;
            assert!(fs::File::open(&path).is_err());
            assert!(fs::OpenOptions::new().write(true).open(&path).is_err());
            assert!(fs::rename(&path,&renamed).is_err());
            Err(io::Error::other("synthetic rollback after contention"))
        },keep_file);
        assert!(result.is_err());assert!(!path.exists() && !renamed.exists());
    }
    #[test] fn competing_creators_produce_one_complete_winner(){
        let path=new_path("race");let barrier=std::sync::Arc::new(std::sync::Barrier::new(2));
        let workers=[b"winner A".to_vec(),b"winner B".to_vec()].into_iter().map(|data|{
            let path=path.clone();let barrier=barrier.clone();
            std::thread::spawn(move||{barrier.wait();save_new(&path,&data).map(|_|data)})
        }).collect::<Vec<_>>();
        let winners=workers.into_iter().filter_map(|w|w.join().unwrap().ok()).collect::<Vec<_>>();
        assert_eq!(winners.len(),1);assert_eq!(fs::read(path).unwrap(),winners[0]);
    }
    #[test] fn os_rolls_back_after_process_exit_without_rust_drop(){
        const FLAG:&str="SR_TEST_SAVE_EXIT_PATH";
        if let Some(path)=std::env::var_os(FLAG){
            let mut f=open_rollback_file(std::path::Path::new(&path)).unwrap();
            f.write_all(b"partial synthetic process output").unwrap();f.sync_all().unwrap();
            std::process::exit(23);
        }
        let path=new_path("process-exit");
        let status=std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact","windows::tests::os_rolls_back_after_process_exit_without_rust_drop","--nocapture"])
            .env(FLAG,&path).status().unwrap();
        assert_eq!(status.code(),Some(23));assert!(!path.exists());
    }
}
