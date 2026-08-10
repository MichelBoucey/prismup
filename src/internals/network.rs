use crate::types::Release;
use colored::Colorize;
use exponential_backoff::Backoff;
use futures_util::TryStreamExt;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::time::{Duration, SystemTime};

pub static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub async fn web_client() -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| {
            format!(
                "{}: {}",
                "Web client build fails".red(),
                e.to_string().red()
            )
            .into()
        })
}

pub async fn get_sha256(
    client: &reqwest::Client,
    url: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let sha256 = client
        .get(url)
        .send()
        .await
        .map_err(|e| {
            format!(
                "{}{}{}: {}",
                "Failed to download SHA256 file'".red(),
                url.red(),
                "'".red(),
                e.to_string().red()
            )
        })?
        .error_for_status()?;
    let sha256 = sha256.text().await?;
    let sha256 = sha256.get(..64).ok_or_else(|| {
        format!(
            "{}{}{}",
            "SHA256 file '".red(),
            url.red(),
            "' has an invalid format".red()
        )
    })?;
    Ok("sha256:".to_owned() + sha256)
}

pub async fn download_backoff(
    client: &reqwest::Client,
    url: &str,
    filepath: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let attempts = 5;
    let min = Duration::from_millis(500);
    let max = Duration::from_secs(60);
    let mut last_error = None;
    for duration in Backoff::new(attempts, min, max) {
        match download(client, url, filepath).await {
            Ok(()) => return Ok(()),
            Err(e) => match duration {
                Some(duration) => {
                    last_error = Some(e);
                    tokio::time::sleep(duration).await;
                }
                None => return Err(e),
            },
        }
    }
    match last_error {
        Some(e) => Err(e),
        None => Err(format!("{}", "Download failed.".red()).into()),
    }
}

pub async fn download(
    client: &reqwest::Client,
    url: &str,
    filepath: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| {
            format!(
                "{}{}{}: {}",
                "Failed to download from '".red(),
                url.red(),
                "': ".red(),
                e.to_string().red()
            )
        })?
        .error_for_status()?;

    let mut file = std::fs::File::create(filepath).map_err(|e| {
        format!(
            "{}{}{}: {}",
            "Failed to create '".red(),
            filepath.red(),
            "': ".red(),
            e.to_string().red()
        )
    })?;
    let mut stream = response.bytes_stream();

    while let Some(bytes) = stream.try_next().await? {
        use std::io::Write;
        file.write_all(&bytes).map_err(|e| {
            format!(
                "{}{}{}: {}",
                "Failed to write stream to '".red(),
                filepath.red(),
                "': ".red(),
                e.to_string().red()
            )
        })?;
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
    let cache_is_stale = match fs::metadata(&filepath) {
        Ok(metadata) => match metadata.modified() {
            Ok(time) => SystemTime::now() > (time + ttl),
            Err(_) => true,
        },
        Err(_) => true,
    };
    if cache_is_stale {
        download_backoff(client, releases_url, &filepath).await?;
    }
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let releases: Vec<Release> = serde_json::from_reader(reader)?;
    Ok(releases)
}
