use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::Cursor;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use antenna_core::Artifact;
use tiny_http::{Header, Response, Server, StatusCode};

use crate::breaker::break_connection;
use crate::support::{REPO, REVISION};

const OK: u16 = 200;
const PARTIAL_CONTENT: u16 = 206;
const FOUND: u16 = 302;
const NOT_FOUND: u16 = 404;
const SERVICE_UNAVAILABLE: u16 = 503;
const TEMPORARY_REDIRECT: u16 = 307;

pub(crate) fn hub_path(artifact: &Artifact) -> String {
    format!("/{REPO}/resolve/{REVISION}/{}", artifact.path)
}

/// How a route answers a request.
pub(crate) enum Behavior {
    Serve,
    FailFirst(u32),
    Status(u16),
    IgnoreRange,
    /// Answers a range request with a `Content-Range` that starts at byte 0.
    MisplacedRange,
    Redirect(String),
    /// Closes the connection after this many body bytes, on the first `times` requests.
    CloseAfter {
        bytes: usize,
        times: u32,
    },
}

pub(crate) struct Route {
    pub(crate) path: String,
    pub(crate) body: Vec<u8>,
    pub(crate) behavior: Behavior,
}

#[derive(Clone)]
pub(crate) struct Request {
    pub(crate) path: String,
    pub(crate) range: Option<String>,
    pub(crate) user_agent: Option<String>,
}

pub(crate) struct TestServer {
    endpoint: String,
    requests: flume::Receiver<Request>,
    seen: Cell<Vec<Request>>,
    server: Arc<Server>,
    thread: Option<JoinHandle<()>>,
}

impl TestServer {
    pub(crate) fn start(routes: Vec<Route>) -> Self {
        let server = Arc::new(Server::http("127.0.0.1:0").unwrap());
        let endpoint = format!("http://{}", server.server_addr().to_ip().unwrap());
        let (sender, requests) = flume::unbounded();
        let state = State::new(routes, endpoint.clone(), sender);
        let listener = Arc::clone(&server);
        let thread = thread::Builder::new()
            .name("antenna-test-server".to_owned())
            .spawn(move || state.serve(&listener))
            .unwrap();
        Self {
            endpoint,
            requests,
            seen: Cell::new(Vec::new()),
            server,
            thread: Some(thread),
        }
    }

    pub(crate) fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub(crate) fn requests(&self, path: &str) -> Vec<Request> {
        let mut seen = self.seen.take();
        seen.extend(self.requests.try_iter());
        let matching = seen
            .iter()
            .filter(|request| request.path == path)
            .cloned()
            .collect();
        self.seen.set(seen);
        matching
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

struct State {
    bodies: BTreeMap<String, Vec<u8>>,
    behaviors: BTreeMap<String, Behavior>,
    endpoint: String,
    log: flume::Sender<Request>,
}

impl State {
    fn new(routes: Vec<Route>, endpoint: String, log: flume::Sender<Request>) -> Self {
        let mut bodies = BTreeMap::new();
        let mut behaviors = BTreeMap::new();
        for route in routes {
            bodies.insert(route.path.clone(), route.body);
            behaviors.insert(route.path, route.behavior);
        }
        Self {
            bodies,
            behaviors,
            endpoint,
            log,
        }
    }

    fn serve(mut self, server: &Server) {
        for request in server.incoming_requests() {
            self.answer(request);
        }
    }

    fn answer(&mut self, request: tiny_http::Request) {
        let path = request.url().to_owned();
        let range = header(&request, "Range");
        let entry = Request {
            path: path.clone(),
            range: range.clone(),
            user_agent: header(&request, "User-Agent"),
        };
        self.log.send(entry).unwrap();
        let response = self.reply(&path, range.as_deref());
        request.respond(response).unwrap();
    }

    fn reply(&mut self, path: &str, range: Option<&str>) -> Reply {
        let Some(body) = self.bodies.get(path) else {
            return empty(NOT_FOUND);
        };
        let behavior = self.behaviors.get_mut(path).unwrap();
        match behavior {
            Behavior::Serve | Behavior::FailFirst(0) => ranged(body, range),
            Behavior::IgnoreRange => Response::from_data(body.clone()).with_status_code(OK),
            Behavior::MisplacedRange => misplaced(body, range),
            Behavior::Status(status) => empty(*status),
            Behavior::FailFirst(remaining) => {
                *remaining -= 1;
                empty(SERVICE_UNAVAILABLE)
            }
            Behavior::Redirect(target) => redirect(FOUND, &format!("{}{target}", self.endpoint)),
            Behavior::CloseAfter { bytes, times } => {
                let location = break_connection(path, body.clone(), *bytes, self.log.clone());
                *times -= 1;
                if *times == 0 {
                    *behavior = Behavior::Serve;
                }
                redirect(TEMPORARY_REDIRECT, &location)
            }
        }
    }
}

fn header(request: &tiny_http::Request, name: &'static str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv(name))
        .map(|header| header.value.to_string())
}

type Reply = Response<Cursor<Vec<u8>>>;

fn empty(status: u16) -> Reply {
    Response::from_data(Vec::new()).with_status_code(status)
}

fn redirect(status: u16, location: &str) -> Reply {
    let location = Header::from_bytes("Location", location).unwrap();
    empty(status).with_header(location)
}

pub(crate) fn parse_range(range: &str) -> (usize, usize) {
    let (start, end) = range
        .strip_prefix("bytes=")
        .unwrap()
        .split_once('-')
        .unwrap();
    (start.parse().unwrap(), end.parse().unwrap())
}

fn ranged(body: &[u8], range: Option<&str>) -> Reply {
    let Some(range) = range else {
        return Response::from_data(body.to_vec()).with_status_code(OK);
    };
    let (start, end) = parse_range(range);
    partial_content(body, start..=end, start)
}

fn misplaced(body: &[u8], range: Option<&str>) -> Reply {
    let Some(range) = range else {
        return ranged(body, None);
    };
    let (start, end) = parse_range(range);
    partial_content(body, start..=end, 0)
}

fn partial_content(body: &[u8], sent: RangeInclusive<usize>, claimed_start: usize) -> Reply {
    let content_range = format!("bytes {claimed_start}-{}/{}", sent.end(), body.len());
    Response::from_data(body[sent].to_vec())
        .with_status_code(StatusCode(PARTIAL_CONTENT))
        .with_header(Header::from_bytes("Content-Range", content_range).unwrap())
}
