use anyhow::Result;
use reqwest::{
    blocking::{Client, Response},
    StatusCode,
};
use std::path::Path;

pub(crate) fn uid(owner: &str, repo: &str, refs: &str, client: &Client) -> Result<String> {
    fn extract_etag(response: Response) -> Option<String> {
        response
            .headers()
            .get("etag")
            .map(|x| x.to_str().map(|x| x.replace('"', "").into()).ok())
            .flatten()
    }

    fn etag_or_ts(response: Response) -> String {
        extract_etag(response).unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string()
        })
    }

    let response = client
        .head(format!("https://github.com/{owner}/{repo}/archive/refs/heads/{refs}.zip"))
        .send()?;
    match response.status() {
        StatusCode::OK => Ok(etag_or_ts(response)),
        StatusCode::NOT_FOUND => {
            let response = client
                .head(format!("https://github.com/{owner}/{repo}/archive/refs/tags/{refs}.zip"))
                .send()?;

            match response.status() {
                StatusCode::OK => Ok(etag_or_ts(response)),
                _ => anyhow::bail!(
                    "Response with StatusCode {} which cannot be handled",
                    response.status()
                ),
            }
        }
        _ => {
            anyhow::bail!("Response with StatusCode {} which cannot be handled", response.status())
        }
    }
}

pub(crate) fn download(
    owner: &str,
    refs: &str,
    repo: &str,
    file_path: &Path,
    client: &Client,
) -> Result<()> {
    let mut zip_file = std::fs::File::create(file_path)?;

    let mut response = client
        .get(format!("https://github.com/{owner}/{repo}/archive/refs/heads/{refs}.zip"))
        .send()?;
    match response.status() {
        StatusCode::OK => {
            response.copy_to(&mut zip_file)?;
            Ok(())
        }
        StatusCode::NOT_FOUND => {
            let mut response = client
                .get(format!("https://github.com/{owner}/{repo}/archive/refs/tags/{refs}.zip"))
                .send()?;
            match response.status() {
                StatusCode::OK => {
                    response.copy_to(&mut zip_file)?;
                    Ok(())
                }
                _ => anyhow::bail!(
                    "Response with StatusCode {} which cannot be handled",
                    response.status()
                ),
            }
        }
        _ => {
            anyhow::bail!("Response with StatusCode {} which cannot be handled", response.status())
        }
    }
}
