use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

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
                let response = handle_request(&request, &root_dir);
                stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                    eprintln!("Failed to write response : {}", e);
                });
            }
        }
        Err(e) => eprintln!("Failed to read from stream: {}", e),
    }
}

fn handle_request(request: &str, root_dir: &str) -> String {
    let (method, url) = parse_request(request);
    match url {
        "/" => respond_with_text("Welcome to the HTTP server"),
        path if path.starts_with("/echo/") => {
            let content = path.trim_start_matches("/echo/");
            if let Some(content_encoding) = extract_accept_encoding(request) {
                println!("content encoding {:?}",content_encoding);
                if content_encoding.contains(&"gzip".to_string())
                {
                respond_with_text_and_content_encoding(&content, &"gzip")
                }
                else {
                    respond_with_text(content)
                }
            } else {
                respond_with_text(content)
            }
        }
        path if path.starts_with("/user-agent") => {
            if let Some(user_agent) = extract_user_agent(request) {
                respond_with_text(&user_agent)
            } else {
                respond_with_status(400, "Bad Request")
            }
        }
        path if path.starts_with("/files/") => {
            let filename = path.trim_start_matches("/files/");
            handle_file_request(method, filename, root_dir, request)
        }
        _ => respond_with_status(404, "Not Found"),
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

fn handle_file_request(method: &str, filename: &str, root_dir: &str, request: &str) -> String {
    let file_path = format!("{}/{}", root_dir, filename);

    match method {
        "POST" => {
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
            if let Err(e) = write_to_file(&file_path, body) {
                eprintln!("Failed to write to file: {}", e);
                respond_with_status(500, "Internal Server Error")
            } else {
                respond_with_status(201, "Created")
            }
        }
        "GET" => {
            if let Ok(content) = read_file(&file_path) {
                respond_with_file(&content)
            } else {
                respond_with_status(404, "Not Found")
            }
        }
        _ => respond_with_status(405, "Method Not Allowed"),
    }
}

fn respond_with_text(content: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        content.len(),
        content
    )
}
fn respond_with_text_and_content_encoding(content: &str,content_encoding: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain \r\nContent-Encoding: {}\r\nContent-Length: {}\r\n\r\n{}",
        content_encoding,
        content.len(),
        content
    )
}
fn respond_with_file(content: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
        content.len(),
        content
    )
}

fn respond_with_status(status: u16, message: &str) -> String {
    format!("HTTP/1.1 {} {}\r\n\r\n", status, message)
}

fn read_file(path: &str) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn write_to_file(path: &str, content: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).write(true).open(path)?;
    file.write_all(content.as_bytes())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let address = "127.0.0.1:4221".to_string();
    let mut root_dir = "";
    if args.len() >= 3
    {
    root_dir = &args[2];
    }
    let server = HttpServer::new(&address, root_dir);
    server.start();
}
