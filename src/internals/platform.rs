use semver::Version;
use std::process::Command;

pub const LINUX: &str = "Linux";
pub const DARWIN: &str = "Darwin";
pub const LINUX_ARCHIVE_OS: &str = "unknown-linux-gnu";
pub const MACOS_ARCHIVE_OS: &str = "apple-darwin";
pub const X86_64_ARCHIVE_ARCH: &str = "x86_64";
pub const AARCH64_ARCHIVE_ARCH: &str = "aarch64";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformOs {
    Linux,
    MacOs,
}

impl PlatformOs {
    fn from_uname(uname_os: &str) -> Result<Self, String> {
        match uname_os {
            LINUX => Ok(PlatformOs::Linux),
            DARWIN => Ok(PlatformOs::MacOs),
            &_ => Err(format!(
                "The operating system {} is not supported by PrismUp.",
                uname_os
            )),
        }
    }

    fn archive_os(self) -> &'static str {
        match self {
            PlatformOs::Linux => LINUX_ARCHIVE_OS,
            PlatformOs::MacOs => MACOS_ARCHIVE_OS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformArch {
    X86_64,
    AArch64,
}

impl PlatformArch {
    fn from_uname(uname_arch: &str) -> Result<Self, String> {
        match uname_arch {
            X86_64_ARCHIVE_ARCH => Ok(PlatformArch::X86_64),
            "arm64" | AARCH64_ARCHIVE_ARCH => Ok(PlatformArch::AArch64),
            &_ => Err(format!(
                "The architecture {} is not supported by PrismUp.",
                uname_arch
            )),
        }
    }

    fn archive_arch(self) -> &'static str {
        match self {
            PlatformArch::X86_64 => X86_64_ARCHIVE_ARCH,
            PlatformArch::AArch64 => AARCH64_ARCHIVE_ARCH,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    os: PlatformOs,
    arch: PlatformArch,
}

impl Platform {
    pub fn from_uname(uname_os: &str, uname_arch: &str) -> Result<Self, String> {
        Ok(Platform {
            os: PlatformOs::from_uname(uname_os)?,
            arch: PlatformArch::from_uname(uname_arch)?,
        })
    }

    pub fn current() -> Result<Self, String> {
        let uname = Command::new("uname")
            .args(["-s", "-m"])
            .output()
            .map_err(|e| format!("Failed to run 'uname -s -m': {}", e))?;
        if !uname.status.success() {
            return Err("'uname -s -m' failed.".to_string());
        }
        let uname_output = String::from_utf8_lossy(&uname.stdout);
        let mut uname_fields = uname_output.split_whitespace();
        let uname_os = uname_fields
            .next()
            .ok_or_else(|| "Failed to get the operating system name from 'uname'.".to_string())?;
        let uname_arch = uname_fields
            .next()
            .ok_or_else(|| "Failed to get the architecture from 'uname'.".to_string())?;
        Platform::from_uname(uname_os, uname_arch)
    }

    pub fn target(&self) -> String {
        format!("{}-{}", self.arch.archive_arch(), self.os.archive_os())
    }

    pub fn asset_name(&self, version: &Version) -> String {
        format!("prism-{}-{}", version, self.target())
    }

    pub fn archive_filename(&self, version: &Version) -> String {
        format!("{}.tar.gz", self.asset_name(version))
    }

    pub fn archive_url(&self, version: &Version) -> String {
        format!(
            "https://github.com/sdiehl/prism/releases/download/v{}/{}",
            version,
            self.archive_filename(version)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internals::versions::to_semver;

    fn version(semver: &str) -> Version {
        to_semver(semver).expect("the version should be a valid semver")
    }

    #[test]
    fn from_uname_maps_linux_x86_64() {
        let platform = Platform::from_uname(LINUX, X86_64_ARCHIVE_ARCH)
            .expect("Linux on x86_64 should be supported");
        assert_eq!(platform.target(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn from_uname_maps_darwin_aarch64() {
        let platform =
            Platform::from_uname(DARWIN, "arm64").expect("macOS on arm64 should be supported");
        assert_eq!(platform.target(), "aarch64-apple-darwin");
    }

    #[test]
    fn from_uname_maps_linux_arm64_to_aarch64() {
        let platform =
            Platform::from_uname(LINUX, "arm64").expect("Linux on arm64 should be supported");
        assert_eq!(platform.target(), "aarch64-unknown-linux-gnu");
    }

    #[test]
    fn from_uname_rejects_unsupported_operating_system() {
        assert!(Platform::from_uname("FreeBSD", X86_64_ARCHIVE_ARCH).is_err());
    }

    #[test]
    fn from_uname_rejects_unsupported_architecture() {
        assert!(Platform::from_uname(LINUX, "riscv64").is_err());
    }

    #[test]
    fn archive_names_contain_the_version_and_the_target() {
        let platform = Platform::from_uname(LINUX, X86_64_ARCHIVE_ARCH)
            .expect("Linux on x86_64 should be supported");
        assert_eq!(
            platform.asset_name(&version("0.22.0")),
            "prism-0.22.0-x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            platform.archive_filename(&version("0.22.0")),
            "prism-0.22.0-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(
            platform.archive_url(&version("0.22.0")),
            "https://github.com/sdiehl/prism/releases/download/v0.22.0/prism-0.22.0-x86_64-unknown-linux-gnu.tar.gz"
        );
    }

    #[test]
    fn current_describes_the_running_platform() {
        let platform = Platform::current().expect("the running platform should be supported");
        assert!(
            platform.target().ends_with(LINUX_ARCHIVE_OS)
                || platform.target().ends_with(MACOS_ARCHIVE_OS)
        );
    }
}
