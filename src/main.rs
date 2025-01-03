use std::fs::File;
use std::io::Write;
use std::io::Read;
#[allow(unused_imports)]
use std::net::TcpListener;
use core::str;
use std::path::Path;
use std::thread;
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                thread::spawn(move || {
                println!("accepted new connection");
                let mut buf = [0; 512];
                _stream.read(&mut buf).unwrap();
                let request = str::from_utf8(&buf).unwrap();
                let parts : Vec<&str> = request.split(" ").collect();
                let url = parts[1];
                println!("url base {:?}",url);
                if url.eq_ignore_ascii_case("/") 
                {
                    _stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
                }
                else if url.starts_with("/echo")
                {
                    let string_contents : Vec<&str> = url.split("/echo/").collect();
                    let content = string_contents[1];
                    println!("contents value {:?}",string_contents);
                    println!("content {:?}",content);
                    println!("url {:?}",url);
                    _stream.write(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",content.len(),content).as_bytes()).unwrap();
                }
                else if url.starts_with("/user-agent")
                {
                    let request_parts:Vec<&str> = request.split("\r\n").collect();
                    println!("request parts {:?}",request_parts);
                    for part in request_parts  {
                        println!("part {:?}",part);
                        if part.to_ascii_lowercase().starts_with("user-agent")
                        {
                            let content: Vec<&str> = part.split(" ").collect();
                            println!("{:?}",content);
                            let header_value= content[1];
                            println!("{:?}",header_value);
                            _stream.write(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",header_value.len(),header_value).as_bytes()).unwrap();
                        }
                    }
                }
                else if url.starts_with("/files")
                {
                    let string_contents : Vec<&str> = url.split("/files/").collect();
                    let file_name = string_contents[1];
                    println!("contents value {:?}",string_contents);
                    println!("fileName {:?}",file_name);
                    println!("url {:?}",url);
                    if Path::new(file_name).exists() 
                    {
                        let mut file_content:String = String::new();
                        let _ = File::open(file_name).unwrap().read_to_string(&mut file_content);
                        println!("file contents {:?}",file_content);
                        _stream.write(format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",file_content.len(),file_content).as_bytes()).unwrap();
                    }
                    else {
                        _stream.write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes()).unwrap();
                    }
                }
                else {
                    _stream.write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes()).unwrap();
                }
            });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
