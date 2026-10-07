use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use antenna_core::{Artifact, Extent};
use ureq::http::{Response, StatusCode};
use ureq::{Agent, Body};

use super::{Fetch, FetchError, copy_blocks, finish_copy, start_offset};
use crate::ModelError;
use crate::layout::file_len;

const HUB_ENDPOINT: &str = "https://huggingface.co";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const MISSING_STATUSES: [u16; 3] = [401, 403, 404];
const TRANSIENT_STATUSES: [u16; 6] = [408, 429, 500, 502, 503, 504];
const USER_AGENT: &str = concat!("antenna/", env!("CARGO_PKG_VERSION"));

/// Downloads artifacts from a Hugging Face server with HTTP range requests.
pub(crate) struct HubFetch {
    agent: Agent,
    endpoint: String,
}

impl HubFetch {
    pub(crate) fn new(endpoint: Option<&str>) -> Self {
        let config = Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_recv_response(Some(RESPONSE_TIMEOUT))
            .user_agent(USER_AGENT)
            .build();
        Self {
            agent: config.into(),
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
        let mut output = open_partial(artifact, partial, response.status(), range.as_deref())?;
        let mut body = response.into_body();
        let result = copy_blocks(&mut body.as_reader(), &mut output, progress, cancel);
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

fn range_header(extent: Extent, have: u64) -> Option<String> {
    let start = start_offset(extent);
    let is_range = matches!(extent, Extent::Range { .. });
    let end = (start + extent.bytes()).saturating_sub(1);
    (have > 0 || is_range).then(|| format!("bytes={}-{end}", start + have))
}

fn open_partial(
    artifact: &Artifact,
    partial: &Path,
    status: StatusCode,
    range: Option<&str>,
) -> Result<File, FetchError> {
    let open = if status == StatusCode::PARTIAL_CONTENT {
        OpenOptions::new().create(true).append(true).open(partial)
    } else if range.is_some() && matches!(artifact.extent, Extent::Range { .. }) {
        return Err(FetchError::Permanent(ModelError::RangeUnsupported {
            artifact: artifact.key,
        }));
    } else {
        File::create(partial)
    };
    open.map_err(FetchError::io(partial))
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert_eq!(is_transient(&ureq::Error::StatusCode(status)), expected);
        }
    }

    #[test]
    fn is_transient_is_false_when_uri_is_bad() {
        assert!(!is_transient(&ureq::Error::BadUri(String::new())));
    }
}
