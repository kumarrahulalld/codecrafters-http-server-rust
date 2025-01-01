use std::io::Write;
use std::io::Read;
#[allow(unused_imports)]
use std::net::TcpListener;
use core::str;
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                let mut buf = [0; 512];
                _stream.read(&mut buf).unwrap();
                let request = str::from_utf8(&buf).unwrap();
                let parts : Vec<&str> = request.split(' ').collect();
                print!("{:?}",parts);
                let url = parts[1];
                print!("{}",url);
                if url.is_empty() 
                {
                    _stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
                }
                else {
                    _stream.write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes()).unwrap();
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
