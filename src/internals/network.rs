use crate::internals::paths::{Layout, PART_EXTENSION, part_path_of};
use crate::types::{NetworkOptions, Release};
use exponential_backoff::Backoff;
use futures_util::TryStreamExt;
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::time::{Duration, SystemTime};

pub static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub const RELEASES_URL: &str = "https://api.github.com/repos/sdiehl/prism/releases";
pub const RELEASES_PAGE_SIZE: u32 = 100;
pub const SHA256_PREFIX: &str = "sha256:";
pub const SHA256_HEX_LEN: usize = 64;

const RELEASES_CACHE_TTL: Duration = Duration::from_secs(3600);
const DOWNLOAD_ATTEMPTS: u32 = 5;
const DOWNLOAD_MIN_RETRY_DELAY: Duration = Duration::from_millis(500);
const DOWNLOAD_MAX_RETRY_DELAY: Duration = Duration::from_secs(60);
const PROGRESS_STEP_PERCENT: u64 = 10;

pub async fn web_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Web client build fails: {}", e))
}

pub fn github_token() -> Option<String> {
    ["GITHUB_TOKEN", "GH_TOKEN"]
        .iter()
        .find_map(|variable| env::var(variable).ok())
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
}

pub fn expected_sha256_from_digest(digest: &str) -> Option<String> {
    let digest = digest.trim();
    let hexadecimal = digest.strip_prefix(SHA256_PREFIX)?;
    (hexadecimal.len() == SHA256_HEX_LEN && hexadecimal.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| digest.to_owned())
}

pub async fn get_sha256(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let sha256 = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to download the SHA256 file '{}': {}", url, e))?
        .error_for_status()
        .map_err(|e| format!("Failed to download the SHA256 file '{}': {}", url, e))?;
    let sha256 = sha256
        .text()
        .await
        .map_err(|e| format!("Failed to read the SHA256 file '{}': {}", url, e))?;
    let sha256 = sha256.trim();
    expected_sha256_from_digest(sha256).ok_or_else(|| {
        format!(
            "The SHA256 file '{}' has an invalid format, it should be '{}' followed by {} hexadecimal characters.",
            url, SHA256_PREFIX, SHA256_HEX_LEN
        )
    })
}

pub async fn download_backoff(
    client: &reqwest::Client,
    url: &str,
    filepath: &Path,
) -> Result<(), String> {
    let mut last_error = None;
    for duration in Backoff::new(
        DOWNLOAD_ATTEMPTS,
        DOWNLOAD_MIN_RETRY_DELAY,
        DOWNLOAD_MAX_RETRY_DELAY,
    ) {
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
    Err(last_error.unwrap_or_else(|| "Download failed.".to_string()))
}

pub async fn download(client: &reqwest::Client, url: &str, filepath: &Path) -> Result<(), String> {
    let part_path = part_path_of(filepath);
    let download_result = download_to(client, url, &part_path).await;
    match download_result {
        Ok(()) => fs::rename(&part_path, filepath).map_err(|e| {
            format!(
                "Failed to move '{}' to '{}': {}",
                part_path.display(),
                filepath.display(),
                e
            )
        }),
        Err(e) => {
            let _ = fs::remove_file(&part_path);
            Err(e)
        }
    }
}

async fn download_to(client: &reqwest::Client, url: &str, part_path: &Path) -> Result<(), String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to download from '{}': {}", url, e))?
        .error_for_status()
        .map_err(|e| format!("Failed to download from '{}': {}", url, e))?;

    let total_bytes = response.content_length();
    let filename = downloaded_file_name_of(part_path);
    let mut file = File::create(part_path)
        .map_err(|e| format!("Failed to create '{}': {}", part_path.display(), e))?;
    let mut stream = response.bytes_stream();
    let mut downloaded_bytes: u64 = 0;
    let mut last_progress_step: u64 = 0;
    while let Some(bytes) = stream
        .try_next()
        .await
        .map_err(|e| format!("Failed to download from '{}': {}", url, e))?
    {
        file.write_all(&bytes)
            .map_err(|e| format!("Failed to write to '{}': {}", part_path.display(), e))?;
        downloaded_bytes += bytes.len() as u64;
        if let Some(total_bytes) = total_bytes.filter(|total_bytes| *total_bytes > 0) {
            let percent = downloaded_bytes.saturating_mul(100) / total_bytes;
            let progress_step = percent / PROGRESS_STEP_PERCENT * PROGRESS_STEP_PERCENT;
            if progress_step > last_progress_step {
                last_progress_step = progress_step;
                eprint!("\rDownloading {}... {}%", filename, progress_step);
            }
        }
    }
    file.flush()
        .map_err(|e| format!("Failed to write to '{}': {}", part_path.display(), e))?;
    drop(file);
    eprintln!(
        "\rDownloaded {} ({}).",
        filename,
        human_size(downloaded_bytes)
    );
    Ok(())
}

pub async fn get_prism_releases(
    client: &reqwest::Client,
    layout: &Layout,
    options: NetworkOptions,
) -> Result<Vec<Release>, String> {
    let releases_cache = layout.releases_cache();
    if !options.offline && (options.force_refresh || releases_cache_is_stale(releases_cache)) {
        let releases = fetch_prism_releases(client).await?;
        write_releases_cache(releases_cache, &releases)?;
    }
    read_releases_cache(releases_cache, options.offline)
}

fn releases_cache_is_stale(releases_cache: &Path) -> bool {
    fs::metadata(releases_cache)
        .and_then(|metadata| metadata.modified())
        .map(|modified_time| SystemTime::now() > modified_time + RELEASES_CACHE_TTL)
        .unwrap_or(true)
}

fn read_releases_cache(releases_cache: &Path, offline: bool) -> Result<Vec<Release>, String> {
    let file = File::open(releases_cache).map_err(|e| {
        if offline {
            format!(
                "No cached Prism releases available in '{}' while offline: {}",
                releases_cache.display(),
                e
            )
        } else {
            format!("Failed to open '{}': {}", releases_cache.display(), e)
        }
    })?;
    serde_json::from_reader(BufReader::new(file)).map_err(|e| {
        format!(
            "Failed to parse the cached Prism releases in '{}': {}",
            releases_cache.display(),
            e
        )
    })
}

fn write_releases_cache(releases_cache: &Path, releases: &[Release]) -> Result<(), String> {
    let part_path = part_path_of(releases_cache);
    let write_result = write_releases(part_path.as_path(), releases).and_then(|_| {
        fs::rename(&part_path, releases_cache).map_err(|e| {
            format!(
                "Failed to move '{}' to '{}': {}",
                part_path.display(),
                releases_cache.display(),
                e
            )
        })
    });
    if write_result.is_err() {
        let _ = fs::remove_file(&part_path);
    }
    write_result
}

fn write_releases(part_path: &Path, releases: &[Release]) -> Result<(), String> {
    let file = File::create(part_path)
        .map_err(|e| format!("Failed to create '{}': {}", part_path.display(), e))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, releases).map_err(|e| {
        format!(
            "Failed to write the Prism releases in '{}': {}",
            part_path.display(),
            e
        )
    })?;
    writer.flush().map_err(|e| {
        format!(
            "Failed to write the Prism releases in '{}': {}",
            part_path.display(),
            e
        )
    })
}

async fn fetch_prism_releases(client: &reqwest::Client) -> Result<Vec<Release>, String> {
    let mut releases: Vec<Release> = Vec::new();
    let mut next_page = Some(format!("{}?per_page={}", RELEASES_URL, RELEASES_PAGE_SIZE));
    while let Some(page_url) = next_page {
        let response = fetch_releases_page(client, &page_url).await?;
        let link_header = response
            .headers()
            .get(reqwest::header::LINK)
            .and_then(|link| link.to_str().ok())
            .map(String::from);
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read the Prism releases: {}", e))?;
        releases.extend(
            serde_json::from_str::<Vec<Release>>(&body)
                .map_err(|e| format!("Failed to parse the Prism releases: {}", e))?,
        );
        next_page = next_page_from_link_header(link_header.as_deref());
    }
    if releases.is_empty() {
        return Err("No Prism release got from Github.".to_string());
    }
    Ok(releases)
}

async fn fetch_releases_page(
    client: &reqwest::Client,
    url: &str,
) -> Result<reqwest::Response, String> {
    let mut request = client.get(url);
    if let Some(token) = github_token() {
        request = request.bearer_auth(token);
    }
    request
        .send()
        .await
        .map_err(|e| format!("Failed to get the Prism releases from Github: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Failed to get the Prism releases from Github: {}", e))
}

pub fn next_page_from_link_header(link_header: Option<&str>) -> Option<String> {
    link_header?
        .split(',')
        .filter(|link| link.contains("rel=\"next\""))
        .filter_map(|link| {
            let url = link.split('<').nth(1)?.split('>').next()?;
            Some(url.trim().to_owned())
        })
        .next()
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|file_name| file_name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn downloaded_file_name_of(part_path: &Path) -> String {
    let file_name = file_name_of(part_path);
    let part_suffix = format!(".{PART_EXTENSION}");
    file_name
        .strip_suffix(part_suffix.as_str())
        .map(str::to_owned)
        .unwrap_or(file_name)
}

fn human_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    let size = bytes as f64;
    if size >= MIB {
        format!("{:.1} MB", size / MIB)
    } else {
        format!("{:.1} kB", size / KIB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internals::paths::temp_layout;

    const DIGEST: &str = "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

    #[test]
    fn expected_sha256_from_digest_accepts_a_complete_digest() {
        assert_eq!(expected_sha256_from_digest(DIGEST).as_deref(), Some(DIGEST));
        assert_eq!(
            expected_sha256_from_digest(&format!("{}\n", DIGEST)).as_deref(),
            Some(DIGEST)
        );
    }

    #[test]
    fn expected_sha256_from_digest_rejects_incomplete_digests() {
        assert_eq!(expected_sha256_from_digest(""), None);
        assert_eq!(expected_sha256_from_digest("   "), None);
        assert_eq!(expected_sha256_from_digest("9f86d081"), None);
        assert_eq!(
            expected_sha256_from_digest(&format!("md5:{}", &DIGEST[7..])),
            None
        );
        assert_eq!(
            expected_sha256_from_digest(&format!("sha256:{}", &DIGEST[7..33])),
            None
        );
        assert_eq!(
            expected_sha256_from_digest(&format!("sha256:{}", "z".repeat(64))),
            None
        );
    }

    #[test]
    fn next_page_from_link_header_reads_the_next_page() {
        assert_eq!(
            next_page_from_link_header(Some(
                "<https://api.github.com/repositories/1/releases?page=2>; rel=\"next\", <https://api.github.com/repositories/1/releases?page=3>; rel=\"last\""
            ))
            .as_deref(),
            Some("https://api.github.com/repositories/1/releases?page=2")
        );
    }

    #[test]
    fn next_page_from_link_header_handles_missing_next_pages() {
        assert_eq!(next_page_from_link_header(None), None);
        assert_eq!(next_page_from_link_header(Some("")), None);
        assert_eq!(
            next_page_from_link_header(Some(
                "<https://api.github.com/repositories/1/releases?page=1>; rel=\"prev\", <https://api.github.com/repositories/1/releases?page=9>; rel=\"last\""
            )),
            None
        );
    }

    #[tokio::test]
    async fn get_prism_releases_fails_when_offline_without_cache() {
        let layout = temp_layout("offline-without-cache");
        let _ = fs::remove_file(layout.releases_cache());
        let options = NetworkOptions {
            force_refresh: false,
            offline: true,
        };
        let error =
            get_prism_releases(&web_client().await.expect("a web client"), &layout, options)
                .await
                .expect_err("no cached release should be available");
        assert!(error.contains("offline"));
        let _ = fs::remove_dir_all(layout.root().parent().expect("a home directory"));
    }

    #[test]
    fn human_size_reads_kilobytes_and_megabytes() {
        assert_eq!(human_size(512), "0.5 kB");
        assert_eq!(human_size(32 * 1024 * 1024), "32.0 MB");
    }
}
