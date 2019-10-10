use std::path::{Path};
use std::io::Result as ioResult;
use std::hash::Hasher;
use std::hash::Hash;
use std::fs::File;
use std::io::Read;
use std::collections::hash_map::DefaultHasher;

pub fn file_etag(path: &Path) -> ioResult<String> {
    let full_path = path.canonicalize()?;
    let meta      = std::fs::metadata(path)?; 

    let mut hasher = DefaultHasher::default();
    full_path.hash(&mut hasher);
    meta.modified()?.hash(&mut hasher);
    meta.len().hash(&mut hasher);
    //meta.created()?.hash(&mut hasher);

    let mut file = File::open(&full_path)?;
    let mut buffer = vec![0; 2048];
    loop {
        let siz = file.read(&mut buffer)?;
        if siz == 0 {
            break;
        }else{
            hasher.write(&buffer[0..siz]);
        }
    }

    Ok(format!("\"{:x}\"", hasher.finish()))
}

pub fn dir_etag(path: &Path) -> ioResult<String> {
    let full_path = path.canonicalize()?;
    let meta      = std::fs::metadata(path)?; 

    let mut hasher = DefaultHasher::default();
    full_path.hash(&mut hasher);
    meta.modified()?.hash(&mut hasher);
    meta.len().hash(&mut hasher);
    //meta.created()?.hash(&mut hasher);

    Ok(format!("\"{:x}\"", hasher.finish()))
}
