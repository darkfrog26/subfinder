use std::fs;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, BufReader, Write};
use std::mem;
use std::ops::Deref;
use std::path::PathBuf;
use reqwest::Url;

use serde_json::Value;
use serde_json::json;

const HASH_BLK_SIZE: u64 = 65536;

fn main() {
    let api_key = std::env::var("OPEN_SUBTITLES_API_KEY").unwrap();
    process_directory(PathBuf::from("."), &api_key);
}

fn process_directory(path: PathBuf, api_key: &String) {
    let paths = fs::read_dir(path)
        .unwrap()
        .flatten()
        .filter(|p| p.path().is_dir() || p.path().extension().is_some())
        .filter(|p| p.path().is_dir() || p.path().extension().unwrap() == "avi");
    for dir_entry in paths {
        let path = dir_entry.path();
        let mut srt_file = path.clone();
        srt_file.set_extension("srt");

        if path.is_dir() {
            process_directory(path, api_key);
        } else if srt_file.exists() {
            println!("{} already exists, skipping!", srt_file.file_name().unwrap().to_str().unwrap());
        } else {
            println!("Attempting to get subtitles for {}", path.to_str().unwrap());
            let file_size = fs::metadata(path.clone()).unwrap().len();
            let file = File::open(path.clone()).unwrap();
            let hash = create_hash(file, file_size).unwrap();
            println!("Hash for {} is {}", path.to_str().unwrap(), hash);

            let ids = file_ids(&hash, api_key);

            match ids.first() {
                Some(id) => {
                    download(*id, srt_file, api_key);
                }
                None => println!("No ids found for {}", &hash)
            }
        }
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

fn file_ids(hash: &String, api_key: &String) -> Vec<u64> {
    let url = Url::parse_with_params("https://api.opensubtitles.com/api/v1/subtitles",
                                     &[("moviehash", hash.as_str()), ("languages", "en"), ("ai_translated", "include"), ("machine_translated", "include")]).unwrap();
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url)
        .header("Api-Key", api_key)
        .header("Content-Type", "application/json")
        .send()
        .unwrap()
        .json::<Value>()
        .unwrap();
    let data = response["data"].as_array().unwrap();
    let ids: Vec<u64> = data.iter().map(|d| {
        let file = d["attributes"]["files"].as_array().unwrap()[0].as_object().unwrap();
        file.get("file_id").unwrap().as_u64().unwrap()
    }).collect();
    ids
}

fn download(file_id: u64, file_name: PathBuf, api_key: &String) {
    let url = Url::parse("https://api.opensubtitles.com/api/v1/download").unwrap();
    let client = reqwest::blocking::Client::new();
    let request: Value = json!({
        "file_id": file_id
    });
    let response = client
        .post(url)
        .header("Api-Key", api_key)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .unwrap()
        .json::<Value>()
        .unwrap();
    let link = response["link"].as_str().unwrap();
    let bytes = client
        .get(link)
        .send()
        .unwrap()
        .bytes()
        .unwrap();
    let mut file = File::create(file_name).unwrap();
    file.write_all(bytes.deref()).unwrap();
}