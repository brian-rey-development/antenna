use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use antenna_core::{Artifact, Extent};
use ureq::http::header::CONTENT_RANGE;
use ureq::http::{Response, StatusCode};
use ureq::unversioned::resolver::DefaultResolver;
use ureq::{Agent, Body};

use super::stall::ReadTimeoutConnector;
use super::{Fetch, FetchError, copy_blocks, finish_copy, start_offset};
use crate::ModelError;
use crate::layout::file_len;

const HUB_ENDPOINT: &str = "https://huggingface.co";
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const READ_TIMEOUT: Duration = Duration::from_secs(30);
const MISSING_STATUSES: [u16; 3] = [401, 403, 404];
const TRANSIENT_STATUSES: [u16; 6] = [408, 429, 500, 502, 503, 504];
const USER_AGENT: &str = concat!("antenna/", env!("CARGO_PKG_VERSION"));

/// Downloads artifacts from a Hugging Face server with HTTP range requests.
pub(crate) struct HubFetch {
    agent: Agent,
    endpoint: String,
}

impl HubFetch {
    pub(crate) fn new(endpoint: Option<&str>, read_timeout: Duration) -> Self {
        let config = Agent::config_builder()
            .timeout_resolve(Some(RESOLVE_TIMEOUT))
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .user_agent(USER_AGENT)
            .build();
        let connector = ReadTimeoutConnector::new(read_timeout);
        Self {
            agent: Agent::with_parts(config, connector, DefaultResolver::default()),
            endpoint: endpoint.unwrap_or(HUB_ENDPOINT).to_owned(),
        }
    }

    fn request(
        &self,
        artifact: &Artifact,
        range: Option<&str>,
    ) -> Result<Response<Body>, FetchError> {
        let url = format!(
            "{}/{}/resolve/{}/{}",
            self.endpoint, artifact.repo, artifact.revision, artifact.path
        );
        let request = self.agent.get(url);
        let request = match range {
            Some(range) => request.header("Range", range),
            None => request,
        };
        request.call().map_err(|error| classify(error, artifact))
    }
}

impl Fetch for HubFetch {
    fn fetch(
        &self,
        artifact: &'static Artifact,
        partial: &Path,
        progress: &dyn Fn(u64),
        cancel: &AtomicBool,
    ) -> Result<(), FetchError> {
        let have = file_len(partial);
        let range = range_header(artifact.extent, have);
        let response = self.request(artifact, range.as_deref())?;
        let (mut output, remaining_bytes) = open_partial(artifact, partial, &response, have)?;
        let mut body = response.into_body();
        let mut reader = body.as_reader().take(remaining_bytes);
        let result = copy_blocks(&mut reader, &mut output, progress, cancel);
        finish_copy(result, partial, |source| {
            FetchError::Transient(ureq::Error::from(source))
        })
    }
}

fn classify(error: ureq::Error, artifact: &Artifact) -> FetchError {
    let key = artifact.key;
    if matches!(error, ureq::Error::StatusCode(status) if MISSING_STATUSES.contains(&status)) {
        return FetchError::Permanent(ModelError::NotFound { artifact: key });
    }
    if is_transient(&error) {
        return FetchError::Transient(error);
    }
    FetchError::Permanent(ModelError::Network {
        artifact: key,
        attempts: 1,
        source: error,
    })
}

fn is_transient(error: &ureq::Error) -> bool {
    if let ureq::Error::StatusCode(status) = error {
        return TRANSIENT_STATUSES.contains(status);
    }
    matches!(
        error,
        ureq::Error::Io(_)
            | ureq::Error::Timeout(_)
            | ureq::Error::ConnectionFailed
            | ureq::Error::HostNotFound
            | ureq::Error::Protocol(_)
    )
}

fn first_byte(extent: Extent, have: u64) -> u64 {
    start_offset(extent) + have
}

fn range_header(extent: Extent, have: u64) -> Option<String> {
    let is_range = matches!(extent, Extent::Range { .. });
    let end = (start_offset(extent) + extent.bytes()).saturating_sub(1);
    (have > 0 || is_range).then(|| format!("bytes={}-{end}", first_byte(extent, have)))
}

fn content_range_start(response: &Response<Body>) -> Option<u64> {
    let value = response.headers().get(CONTENT_RANGE)?.to_str().ok()?;
    let (start, _) = value.strip_prefix("bytes ")?.split_once('-')?;
    start.parse().ok()
}

/// Opens the partial file for the body of the response. Returns the file and the number of body
/// bytes that the download keeps.
fn open_partial(
    artifact: &Artifact,
    partial: &Path,
    response: &Response<Body>,
    have: u64,
) -> Result<(File, u64), FetchError> {
    let range_error = || {
        FetchError::Permanent(ModelError::RangeUnsupported {
            artifact: artifact.key,
        })
    };
    let bytes = artifact.extent.bytes();
    if response.status() == StatusCode::PARTIAL_CONTENT {
        if content_range_start(response) != Some(first_byte(artifact.extent, have)) {
            return Err(range_error());
        }
        let output = OpenOptions::new().create(true).append(true).open(partial);
        return Ok((
            output.map_err(FetchError::io(partial))?,
            bytes.saturating_sub(have),
        ));
    }
    if matches!(artifact.extent, Extent::Range { .. }) {
        return Err(range_error());
    }
    Ok((
        File::create(partial).map_err(FetchError::io(partial))?,
        bytes,
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::AtomicBool;
    use std::thread;

    use super::*;

    const STALL_READ_TIMEOUT: Duration = Duration::from_millis(200);
    const ANNOUNCED_BYTES: u64 = 100_000;
    const SENT_BYTES: usize = 1_000;
    const STALLED: Artifact = Artifact {
        key: "stalled",
        repo: "antenna/test",
        revision: "0123456789abcdef0123456789abcdef01234567",
        path: "stalled.bin",
        extent: Extent::Whole {
            bytes: ANNOUNCED_BYTES,
        },
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    };

    fn stall_after_partial_body(stream: &mut TcpStream, release: &flume::Receiver<()>) {
        for line in BufReader::new(&*stream).lines() {
            if line.unwrap().is_empty() {
                break;
            }
        }
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {ANNOUNCED_BYTES}\r\n\r\n");
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(&[7_u8; SENT_BYTES]).unwrap();
        stream.flush().unwrap();
        release.recv().unwrap();
    }

    #[test]
    fn range_header_is_absent_when_whole_file_starts() {
        assert_eq!(range_header(Extent::Whole { bytes: 100 }, 0), None);
    }

    #[test]
    fn range_header_continues_from_partial_when_whole_file_resumes() {
        let header = range_header(Extent::Whole { bytes: 100 }, 40);

        assert_eq!(header.as_deref(), Some("bytes=40-99"));
    }

    #[test]
    fn range_header_covers_extent_when_range_starts() {
        let extent = Extent::Range {
            offset: 1000,
            bytes: 50,
        };

        assert_eq!(range_header(extent, 0).as_deref(), Some("bytes=1000-1049"));
        assert_eq!(range_header(extent, 20).as_deref(), Some("bytes=1020-1049"));
    }

    #[test]
    fn fetch_times_out_and_keeps_bytes_when_server_stalls_mid_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (release, held) = flume::bounded(1);
        let server = thread::Builder::new()
            .name("antenna-test-stall".to_owned())
            .spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stall_after_partial_body(&mut stream, &held);
            })
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let partial = directory.path().join("stalled.bin.part");
        let fetcher = HubFetch::new(Some(&endpoint), STALL_READ_TIMEOUT);

        let result = fetcher.fetch(&STALLED, &partial, &|_| {}, &AtomicBool::new(false));

        release.send(()).unwrap();
        server.join().unwrap();
        assert!(matches!(
            result,
            Err(FetchError::Transient(ureq::Error::Timeout(_)))
        ));
        assert_eq!(fs::read(&partial).unwrap(), [7_u8; SENT_BYTES]);
    }

    #[test]
    fn is_transient_follows_status_list() {
        let statuses = [
            (408, true),
            (429, true),
            (500, true),
            (502, true),
            (503, true),
            (504, true),
            (400, false),
            (416, false),
            (501, false),
        ];

        for (status, expected) in statuses {
            let error = ureq::Error::StatusCode(status);

            assert_eq!(is_transient(&error), expected, "status {status}");
        }
    }

    #[test]
    fn is_transient_is_false_when_uri_is_bad() {
        assert!(!is_transient(&ureq::Error::BadUri(String::new())));
    }
}
