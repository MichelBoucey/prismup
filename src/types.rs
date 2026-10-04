use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NetworkOptions {
    pub force_refresh: bool,
    pub offline: bool,
}

/// The fields of a Github release used by PrismUp.
///
/// The other fields of the Github payload are ignored, so that a Github
/// change on them cannot break the parsing of the Prism releases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<Assets>,
}

/// An asset of a Github release, without any binary for a platform.
///
/// Github does not give any digest for such an asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assets {
    pub name: String,
    #[serde(default)]
    pub digest: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASE_WITHOUT_OPTIONAL_GITHUB_FIELDS: &str = r#"{
      "tag_name": "v0.23.0",
      "draft": false,
      "prerelease": false,
      "assets": [
        { "name": "prism-0.23.0-x86_64-unknown-linux-gnu.tar.gz", "digest": null },
        {
          "name": "prism-0.23.0-x86_64-unknown-linux-gnu.tar.gz.sha256",
          "digest": "sha256:39e84f0677d722cf15e8142f016f1b39a55640c6d14b5f8120dfbadb5ffa807a"
        }
      ]
    }"#;

    const RELEASE_CACHED_BY_A_PREVIOUS_PRISMUP_VERSION: &str = r#"{
      "url": "https://api.github.com/repos/sdiehl/prism/releases/1",
      "author": { "login": "sdiehl", "id": 1 },
      "tag_name": "v0.22.0",
      "immutable": false,
      "draft": false,
      "prerelease": false,
      "assets": [
        {
          "name": "prism-0.22.0-x86_64-unknown-linux-gnu.tar.gz",
          "label": "",
          "uploader": { "login": "sdiehl", "id": 1 },
          "digest": "sha256:39e84f0677d722cf15e8142f016f1b39a55640c6d14b5f8120dfbadb5ffa807a"
        }
      ],
      "reactions": { "url": "https://api.github.com/reactions", "total_count": 0, "+1": 1 }
    }"#;

    #[test]
    fn a_release_cached_by_a_previous_prismup_version_is_parsed() {
        let release: Release = serde_json::from_str(RELEASE_CACHED_BY_A_PREVIOUS_PRISMUP_VERSION)
            .expect("a release cached by a previous PrismUp version should be parsed");
        assert_eq!(release.tag_name, "v0.22.0");
        assert_eq!(release.assets.len(), 1);
        assert_eq!(
            release.assets[0].name,
            "prism-0.22.0-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert!(release.assets[0].digest.is_some());
    }

    #[test]
    fn a_release_without_the_optional_github_fields_is_parsed() {
        let release: Release = serde_json::from_str(RELEASE_WITHOUT_OPTIONAL_GITHUB_FIELDS)
            .expect("a Github release without its optional fields should be parsed");
        assert_eq!(release.tag_name, "v0.23.0");
        assert!(!release.draft);
        assert!(!release.prerelease);
        assert_eq!(
            release
                .assets
                .iter()
                .map(|asset| asset.name.as_str())
                .collect::<Vec<&str>>(),
            vec![
                "prism-0.23.0-x86_64-unknown-linux-gnu.tar.gz",
                "prism-0.23.0-x86_64-unknown-linux-gnu.tar.gz.sha256"
            ]
        );
        assert_eq!(release.assets[0].digest, None);
        assert!(release.assets[1].digest.is_some());
    }

    #[test]
    fn a_release_without_any_asset_is_parsed() {
        let release: Release = serde_json::from_str(r#"{ "tag_name": "v0.2.0" }"#)
            .expect("a Github release without any asset should be parsed");
        assert_eq!(release.tag_name, "v0.2.0");
        assert!(!release.draft);
        assert!(!release.prerelease);
        assert!(release.assets.is_empty());
    }

    #[test]
    fn an_asset_without_any_digest_is_parsed() {
        let asset: Assets =
            serde_json::from_str(r#"{ "name": "prism-0.2.0-x86_64-unknown-linux-gnu.tar.gz" }"#)
                .expect("a Github asset without any digest should be parsed");
        assert_eq!(asset.name, "prism-0.2.0-x86_64-unknown-linux-gnu.tar.gz");
        assert_eq!(asset.digest, None);
    }
}
