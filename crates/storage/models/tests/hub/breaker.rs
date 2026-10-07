use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::server::{Request, parse_range};

/// Starts a one-shot server that answers one request. It declares the `Content-Length` of the
/// requested range, sends only `count` bytes and closes the connection. `tiny_http` cannot close a
/// connection early. Returns the URL of the one-shot server.
pub(crate) fn break_connection(
    path: &str,
    body: Vec<u8>,
    count: usize,
    log: flume::Sender<Request>,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let location = format!("http://{}{path}", listener.local_addr().unwrap());
    let path = path.to_owned();
    thread::Builder::new()
        .name("antenna-test-breaker".to_owned())
        .spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            cut_response(stream, &path, &body, count, &log);
        })
        .unwrap();
    location
}

fn read_range(stream: &TcpStream) -> Option<String> {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut range = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            return range;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("range:") {
            range = Some(value.trim().to_owned());
        }
    }
}

fn cut_response(
    mut stream: TcpStream,
    path: &str,
    body: &[u8],
    count: usize,
    log: &flume::Sender<Request>,
) {
    let range = read_range(&stream);
    let request = Request {
        path: path.to_owned(),
        range: range.clone(),
        user_agent: None,
    };
    log.send(request).unwrap();
    let (start, end) = range.as_deref().map_or((0, body.len() - 1), parse_range);
    let status = if range.is_some() {
        format!(
            "206 Partial Content\r\nContent-Range: bytes {start}-{end}/{}",
            body.len()
        )
    } else {
        "200 OK".to_owned()
    };
    let length = end - start + 1;
    let head = format!("HTTP/1.1 {status}\r\nContent-Length: {length}\r\n\r\n");
    stream.write_all(head.as_bytes()).unwrap();
    stream
        .write_all(&body[start..start + count.min(length)])
        .unwrap();
    stream.flush().unwrap();
}
