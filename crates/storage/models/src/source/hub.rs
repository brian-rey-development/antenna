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
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const MISSING_STATUSES: [u16; 3] = [401, 403, 404];
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
            .timeout_recv_response(Some(READ_TIMEOUT))
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
    let is_missing =
        matches!(error, ureq::Error::StatusCode(status) if MISSING_STATUSES.contains(&status));
    if is_missing {
        return FetchError::Permanent(ModelError::NotFound {
            artifact: artifact.key,
        });
    }
    FetchError::Transient(error)
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
