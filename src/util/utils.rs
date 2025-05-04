use std::fs;
use std::io::{ Cursor, Read, Result as IoResult };
use std::path::Path;
use encoding_rs::UTF_16LE;
use encoding_rs_io::DecodeReaderBytesBuilder;
use log::info;
use crate::parser::detect_format;


pub fn read_file_content<P: AsRef<Path>>(file_path: P) -> IoResult<String> {
    info!("Reading file: {}", file_path.as_ref().display());
    let raw = fs::read(&file_path)?;
    if raw.starts_with(&[0xff, 0xfe]) {
        let mut decoder = DecodeReaderBytesBuilder::new()
            .encoding(Some(UTF_16LE))
            .bom_override(true)
            .build(Cursor::new(raw));

        let mut content = String::new();
        decoder.read_to_string(&mut content)?;
        Ok(content)
    } else {
        String::from_utf8(raw).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

pub fn read_file_and_detect_format<P: AsRef<Path>>(file_path: P) -> IoResult<(String, String)> {
    let content = read_file_content(&file_path)?;

    info!("Detecting format...");

    let file_path_str = file_path.as_ref().to_str().unwrap_or("unknown_path");
    let format = detect_format(file_path_str, &content);

    info!("Detected format: {}", format);
    info!("Processing {} format file: {}", format, file_path.as_ref().display());

    Ok((content, format))
}

pub fn logo() {
    println!(
        r#"
        ____  ____  ____  _  _  ____   ___ 
        (    \(  _ \(___ \/ )( \(  __) / __)
         ) D ( ) _ ( / __/\ \/ / ) _) ( (__ 
        (____/(____/(____) \__/ (____) \___)                                                                      
        "#
    );
    println!("Database to Vector Migration Tool\n");
}


pub fn init_thread_pool(num_threads: usize) {
    let thread_count = if num_threads == 0 { num_cpus::get() } else { num_threads };
    rayon::ThreadPoolBuilder::new().num_threads(thread_count).build_global().unwrap();
    info!("Using {} threads for parallel processing", thread_count);
}
