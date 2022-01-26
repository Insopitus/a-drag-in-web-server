pub mod mods;
pub mod error;
use std::{
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::Arc,
};

use mods::{
    folder_reader::FolderReader, media_type::MediaType, request_header::RequestHeader,
    response_header::ResponseHeader, thread_pool::ThreadPool,
};

pub fn run(mut port: usize, path: String) {
    loop {
        let start = listen(port, path.clone());
        if let Err(e) = start {
            if e.kind() == io::ErrorKind::AddrInUse {
                port += 1;
                continue;
            } else {
                println!("Server failed to start");
                break;
            }
        } else {
            break;
        }
    }
    println!("Press Enter to continue.");
    std::io::stdin().read_line(&mut String::new()).unwrap();
}

fn listen(port: usize, path: String) -> Result<(), io::Error> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", &port))?;
    let thread_pool = ThreadPool::new(5);
    println!("Server listening at http://localhost:{}", &port);

    // auto start the browser
    std::process::Command::new("cmd.exe")
        .arg("/C")
        .arg("start")
        .arg(format!("http://localhost:{}", port))
        .spawn()
        .ok();

    let media_type_map = Arc::new(MediaType::new());
    let folder_reader = Arc::new(FolderReader::new(Path::new(&path)));
    for stream in listener.incoming() {
        let mut stream = stream?;
        let media_type_map = media_type_map.clone();
        let folder_reader = folder_reader.clone();
        thread_pool.execute(move || {
            let (code, media_type,content_length) =
                handle_connection(&stream, folder_reader, media_type_map.clone());
            send_response_headers(&mut stream, code, &media_type,content_length);
            send_response_body();
        });
        // self.handle_connection(stream)?;
    }
    Ok(())
}

fn handle_connection(
    stream: &TcpStream,
    folder_reader: Arc<FolderReader>,
    media_type_map: Arc<MediaType>,
) -> (u32, String,usize) {
    let mut reader = BufReader::new(stream);
    let mut string = String::with_capacity(1024);

    loop {
        let line_size = reader.read_line(&mut string).unwrap_or(0);
        // println!("line size: {}",&line_size);
        if line_size <= 2 {
            break; //break at the end of the header (an empty line with only b'\r\n')
        }
    }
    
    let header = RequestHeader::new(string);
    if let Some(header) = header {
        let code;
        let path = header.get_path();
        let path = if path == "/" {
            "index.html" // redirect if path is empty
        } else {
            path
        };
        let mime_type;
        let suffix = path.split(".").last();
        if let Some(suffix) = suffix {
            mime_type = media_type_map.get_mime_type(suffix).unwrap_or("");
        } else {
            mime_type = "";
        }
        let mut content_length:usize = 0;
        match folder_reader.get_file_size(path) {
            Ok(length) => {
                code = 200;
                content_length = length.try_into().unwrap_or(0);
            }
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => {
                    code = 404;
                }
                io::ErrorKind::PermissionDenied => {
                    code = 403;
                }
                _ => {
                    code = 403;
                }
            },
        }
        println!("Request: {} - {}", path, code);
        (code, mime_type.to_owned(),content_length)
    } else {
        (400, "".to_owned(),0)
    }
}
fn send_response_headers(stream:&mut TcpStream,code:u32,mime_type:&str,content_length:usize)-> Result<(), std::io::Error>{
    let mut response_header = ResponseHeader::new(code);
    if mime_type != "" {
        response_header.insert_field("Content-Type".to_string(), mime_type.to_string());
    }
    response_header.insert_field("Content-Length".to_string(), content_length.to_string());
    response_header.insert_field("Server".to_string(), "A.D.O.W.S.".to_string());
    let response_header = response_header.to_string();
    let mut response = Vec::with_capacity(response_header.len() + content_length);
    response.append(&mut response_header.as_bytes().into());
    stream.write_all(&response)?;
    stream.flush()?;
    Ok(())
}
fn send_response_body(){

}

