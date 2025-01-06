use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use flate2::write::GzEncoder;
use flate2::Compression;

struct HttpServer {
    address: String,
    root_dir: String,
}

impl HttpServer {
    fn new(address: &str, root_dir: &str) -> Self {
        Self {
            address: address.to_string(),
            root_dir: root_dir.to_string(),
        }
    }

    fn start(&self) {
        let listener = TcpListener::bind(&self.address).expect("Failed to bind to address");
        println!("Server is running on {}", self.address);
        let root_dir = Arc::new(self.root_dir.clone());
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let root_dir = Arc::clone(&root_dir);
                    thread::spawn(move || handle_client(stream, root_dir));
                }
                Err(e) => eprintln!("Failed to accept connection: {}", e),
            }
        }
    }
}

fn handle_client(mut stream: TcpStream, root_dir: Arc<String>) {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(bytes_read) => {
            if let Ok(request) = String::from_utf8(buffer[..bytes_read].to_vec()) {
                handle_request(&request, &root_dir, &mut stream);
            }
        }
        Err(e) => eprintln!("Failed to read from stream: {}", e),
    }
}

fn handle_request(request: &str, root_dir: &str, stream: &mut TcpStream) {
    let (method, url) = parse_request(request);
    match url {
        "/" => {
            let response = respond_with_text("Welcome to the HTTP server");
            stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                eprintln!("Failed to write response: {}", e);
            });
        }
        path if path.starts_with("/echo/") => {
            let content = path.trim_start_matches("/echo/");
            if let Some(content_encoding) = extract_accept_encoding(request) {
                if content_encoding.contains(&"gzip".to_string()) {
                    respond_with_gzip(content, stream);
                } else {
                    let response = respond_with_text(content);
                    stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                        eprintln!("Failed to write response: {}", e);
                    });
                }
            } else {
                let response = respond_with_text(content);
                stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                    eprintln!("Failed to write response: {}", e);
                });
            }
        }
        path if path.starts_with("/user-agent") => {
            if let Some(user_agent) = extract_user_agent(request) {
                let response = respond_with_text(&user_agent);
                stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                    eprintln!("Failed to write response: {}", e);
                });
            } else {
                let response = respond_with_status(400, "Bad Request");
                stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                    eprintln!("Failed to write response: {}", e);
                });
            }
        }
        path if path.starts_with("/files/") => {
            let filename = path.trim_start_matches("/files/");
            handle_file_request(method, filename, root_dir, request, stream);
        }
        _ => {
            let response = respond_with_status(404, "Not Found");
            stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                eprintln!("Failed to write response: {}", e);
            });
        }
    }
}

fn parse_request(request: &str) -> (&str, &str) {
    let lines: Vec<&str> = request.lines().collect();
    if let Some(first_line) = lines.get(0) {
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() >= 2 {
            return (parts[0], parts[1]);
        }
    }
    ("", "")
}

fn extract_user_agent(request: &str) -> Option<String> {
    request
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with("user-agent"))
        .map(|line| line.split(": ").nth(1).unwrap_or("").to_string())
}

fn extract_accept_encoding(request: &str) -> Option<Vec<String>> {
    request
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with("accept-encoding"))
        .and_then(|line| {
            line.split(": ")
                .nth(1) // Get the value part of "Accept-Encoding: ...".
                .map(|value| {
                    value
                        .split(',') // Split encodings by comma.
                        .map(|s| s.trim().to_string()) // Trim and convert each encoding to a String.
                        .collect::<Vec<String>>() // Collect into a Vec<String>.
                })
        })
}

fn handle_file_request(method: &str, filename: &str, root_dir: &str, request: &str, stream: &mut TcpStream) {
    let file_path = format!("{}/{}", root_dir, filename);

    match method {
        "POST" => {
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
            if let Err(e) = write_to_file(&file_path, body) {
                eprintln!("Failed to write to file: {}", e);
                let response = respond_with_status(500, "Internal Server Error");
                stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                    eprintln!("Failed to write response: {}", e);
                });
            } else {
                let response = respond_with_status(201, "Created");
                stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                    eprintln!("Failed to write response: {}", e);
                });
            }
        }
        "GET" => {
            if let Ok(content) = read_file(&file_path) {
                let response = respond_with_file(&content);
                stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                    eprintln!("Failed to write response: {}", e);
                });
            } else {
                let response = respond_with_status(404, "Not Found");
                stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                    eprintln!("Failed to write response: {}", e);
                });
            }
        }
        _ => {
            let response = respond_with_status(405, "Method Not Allowed");
            stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                eprintln!("Failed to write response: {}", e);
            });
        }
    }
}

fn respond_with_text(content: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        content.len(),
        content
    )
}

fn respond_with_gzip(content: &str, stream: &mut TcpStream) {
    let compressed = gzip_compress(content);

    // Write HTTP headers
    let headers = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/plain\r\n\
         Content-Encoding: gzip\r\n\
         Content-Length: {}\r\n\
         \r\n",
        compressed.len()
    );

    // Write headers and gzip content directly to the stream
    if let Err(e) = stream.write_all(headers.as_bytes()) {
        eprintln!("Failed to write headers: {}", e);
    }

    if let Err(e) = stream.write_all(&compressed) {
        eprintln!("Failed to write compressed data: {}", e);
    }
}

fn gzip_compress(input: &str) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(input.as_bytes()).expect("Failed to write to encoder");
    encoder.finish().expect("Failed to finish compression")
}

fn respond_with_status(status: u16, message: &str) -> String {
    format!("HTTP/1.1 {} {}\r\n\r\n", status, message)
}

fn respond_with_file(content: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
        content.len(),
        content
    )
}

fn write_to_file(path: &str, content: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).write(true).open(path)?;
    file.write_all(content.as_bytes())
}

fn read_file(path: &str) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let address = "127.0.0.1:4221".to_string();
    let mut root_dir = "";
    if args.len() >= 3 {
        root_dir = &args[2];
    }
    let server = HttpServer::new(&address, root_dir);
    server.start();
}
