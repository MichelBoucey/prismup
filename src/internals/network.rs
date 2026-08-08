use crate::types::Release;
use futures_util::TryStreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use std::fs;
use std::fs::{File, exists};
use std::io::BufReader;
use std::time::{Duration, SystemTime};

pub static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub async fn web_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Web client build fails")
}

pub async fn get_sha256(
    client: &reqwest::Client,
    url: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let sha256 = client
        .get(url)
        .headers(headers())
        .send()
        .await
        .map_err(|e| format!("Failed to download SHA256 file'{}': {}", url, e))?;
    let sha256 = sha256.text().await?;
    let sha256 = &sha256[0..64];
    Ok("sha256:".to_owned() + sha256)
}

pub async fn download(
    client: &reqwest::Client,
    url: &str,
    filepath: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .get(url)
        .headers(headers())
        .send()
        .await
        .map_err(|e| format!("Failed to download from '{}': {}", url, e))?;

    let mut file = std::fs::File::create(filepath)
        .map_err(|e| format!("Failed to create '{}': {}", filepath, e))?;
    let mut stream = response.bytes_stream();

    while let Ok(Some(bytes)) = stream.try_next().await {
        use std::io::Write;
        file.write_all(&bytes)
            .map_err(|e| format!("Failed to write stream to '{}': {}", filepath, e))?;
    }
    Ok(())
}

pub async fn get_prism_releases(
    client: &reqwest::Client,
    home_dir: &str,
) -> Result<Vec<Release>, Box<dyn std::error::Error>> {
    let releases_url = "https://api.github.com/repos/sdiehl/prism/releases";
    let filepath = home_dir.to_owned() + ".cache/prismup/releases.json";
    let ttl = Duration::new(3600, 0);
    if !exists(&filepath).unwrap() {
        download(client, releases_url, &filepath).await?;
    } else {
        let metadata = fs::metadata(&filepath)?;
        if let Ok(time) = metadata.modified()
            && SystemTime::now() > (time + ttl)
        {
            download(client, releases_url, &filepath).await?;
        };
    }
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let releases: Vec<Release> = serde_json::from_reader(reader)?;
    Ok(releases)
}

fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        "Content-Disposition",
        HeaderValue::from_static("attachment"),
    );
    headers
}
