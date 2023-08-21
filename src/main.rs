use std::fs;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, BufReader};
use std::mem;

const HASH_BLK_SIZE: u64 = 65536;

fn main() {
    let paths = fs::read_dir(".")
        .unwrap()
        .flatten()
        .filter(|p| p.path().extension().is_some())
        .filter(|p| p.path().extension().unwrap() == "avi");
    for dir_entry in paths {
        let path = dir_entry.path();

        let file_size = fs::metadata(path.clone()).unwrap().len();
        let file = File::open(path.clone()).unwrap();
        let hash = create_hash(file, file_size).unwrap();
        println!("Hash for {} is {}", path.to_str().unwrap(), hash);
    }
}

fn create_hash(file: File, fsize: u64) -> Result<String, std::io::Error> {
    let mut buf = [0u8; 8];
    let mut word: u64;

    let mut hash_val: u64 = fsize;  // seed hash with file size

    let iterations = HASH_BLK_SIZE /  8;

    let mut reader = BufReader::with_capacity(HASH_BLK_SIZE as usize, file);

    for _ in 0..iterations {
        reader.read(&mut buf).unwrap();
        unsafe { word = mem::transmute(buf); };
        hash_val = hash_val.wrapping_add(word);
    }

    reader.seek(SeekFrom::Start(fsize - HASH_BLK_SIZE)).unwrap();

    for _ in 0..iterations {
        reader.read(&mut buf).unwrap();
        unsafe { word = mem::transmute( buf); };
        hash_val = hash_val.wrapping_add(word);
    }

    let hash_string = format!("{:01$x}", hash_val, 16);

    Ok(hash_string)
}