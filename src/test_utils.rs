//! This module contains helper functions for tests (aka test dump).
use httpmock::Method::POST;
use httpmock::{Mock, MockServer};
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;
use snafu::{ResultExt, Snafu};
use std::convert::TryFrom;
use std::env;
use std::path::Path;
use std::time::Duration;

use camino::Utf8PathBuf;

use crate::{filen_settings::FilenSettings, utils};

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("Current working directory cannot be accessed: {}", source))]
    CurrentWorkingDirectoryIsUnaccessible { source: std::io::Error },

    #[snafu(display(
        "Caller expected file system path '{}' to be a valid UTF-8 string, but it was not: {}",
        path,
        source
    ))]
    FileSystemPathIsNotUtf8 {
        path: String,
        source: camino::FromPathBufError,
    },
}

pub(crate) fn init_server() -> (MockServer, FilenSettings) {
    let server = MockServer::start();
    let filen_settings = FilenSettings {
        api_servers: vec![Url::parse(&server.base_url()).unwrap()],
        download_servers: vec![Url::parse(&server.base_url()).unwrap()],
        upload_servers: vec![Url::parse(&server.base_url()).unwrap()],
        request_timeout: Duration::from_secs(10),
        upload_chunk_timeout: Duration::from_secs(10),
        download_chunk_timeout: Duration::from_secs(10),
    };
    (server, filen_settings)
}

pub(crate) fn deserialize_from_file<U: DeserializeOwned>(response_file_path: &str) -> U {
    let response_contents = read_project_file(response_file_path);
    serde_json::from_slice(&response_contents).unwrap()
}

pub(crate) fn project_path() -> Result<Utf8PathBuf> {
    match env::var("CARGO_MANIFEST_DIR") {
        Ok(val) => Ok(Utf8PathBuf::from(val)),
        _ => {
            let curr_dir = env::current_dir().context(CurrentWorkingDirectoryIsUnaccessible {})?;
            Utf8PathBuf::try_from(curr_dir.clone()).context(FileSystemPathIsNotUtf8 {
                path: format!("{:?}", curr_dir),
            })
        }
    }
}

pub(crate) fn project_path_for(file_path: &str) -> Utf8PathBuf {
    match Path::new(&file_path).is_absolute() {
        true => Utf8PathBuf::from(file_path),
        false => {
            let mut proj_dir = project_path().expect("Cannot get project path or it contains invalid UTF-8");
            proj_dir.push(file_path);
            proj_dir
        }
    }
}

pub(crate) fn read_project_file(file_path: &str) -> Vec<u8> {
    let target_path = project_path_for(file_path);
    utils::read_file(&target_path).expect(&format!("Cannot read file: {}", target_path))
}

pub(crate) fn setup_json_mock<'a, T: Serialize, U: Serialize>(
    api_path: &str,
    request_payload: &T,
    response_payload: &U,
    server: &'a MockServer,
) -> Mock<'a> {
    server.mock(|when, then| {
        when.method(POST)
            .path(api_path)
            .header("content-type", "application/json")
            .json_body(json!(request_payload));
        then.status(200)
            .header("content-type", "text/html")
            .json_body(json!(response_payload));
    })
}
