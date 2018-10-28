//! Get LLVM/Clang source

use failure::{bail, err_msg};
use log::info;
use reqwest;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempdir::TempDir;
use url::Url;

use crate::error::*;

#[derive(Debug)]
pub enum Resource {
    Svn { url: String },
    Git { url: String, branch: Option<String> },
    Tar { url: String },
}

impl Resource {
    pub fn from_url(url_str: &str) -> Result<Self> {
        // Check file extension
        if let Ok(filename) = get_filename_from_url(url_str) {
            for ext in &[".tar.gz", ".tar.xz", ".tar.bz2", ".tar.Z", ".tgz", ".taz"] {
                if filename.ends_with(ext) {
                    info!("Find archive extension '{}' at the end of URL", ext);
                    return Ok(Resource::Tar {
                        url: url_str.into(),
                    });
                }
            }

            if filename.ends_with("trunk") {
                info!("Find 'trunk' at the end of URL");
                return Ok(Resource::Svn {
                    url: url_str.into(),
                });
            }

            if filename.ends_with(".git") {
                info!("Find '.git' extension");
                return Ok(Resource::Git {
                    url: url_str.into(),
                    branch: None,
                });
            }
        }

        // Hostname
        let url = Url::parse(url_str)?;
        for service in &["github.com", "gitlab.com"] {
            if url.host_str() == Some(service) {
                info!("URL is a cloud git service: {}", service);
                return Ok(Resource::Git {
                    url: url_str.into(),
                    branch: None,
                });
            }
        }

        if url.host_str() == Some("llvm.org") {
            if url.path().starts_with("/svn") {
                info!("URL is LLVM SVN repository");
                return Ok(Resource::Svn {
                    url: url_str.into(),
                });
            }
            if url.path().starts_with("/git") {
                info!("URL is LLVM Git repository");
                return Ok(Resource::Git {
                    url: url_str.into(),
                    branch: None,
                });
            }
        }

        // Try access with git
        //
        // - SVN repository cannot handle git access
        // - Some Git service (e.g. GitHub) *can* handle svn access
        //
        // ```
        // git init
        // git remote add $url
        // git ls-remote       # This must fail for SVN repo
        // ```
        info!("Try access with git to {}", url_str);
        let tmp_dir = TempDir::new("llvmenv-detect-git")?;
        Command::new("git")
            .arg("init")
            .current_dir(tmp_dir.path())
            .silent()
            .check_run()?;
        Command::new("git")
            .args(&["remote", "add", "origin"])
            .arg(url_str)
            .current_dir(tmp_dir.path())
            .silent()
            .check_run()?;
        match Command::new("git")
            .args(&["ls-remote"])
            .current_dir(tmp_dir.path())
            .silent()
            .check_run()
        {
            Ok(_) => {
                info!("Git access succeeds");
                Ok(Resource::Git {
                    url: url_str.into(),
                    branch: None,
                })
            }
            Err(_) => {
                info!("Git access failed. Regarded as a SVN repository.");
                Ok(Resource::Svn {
                    url: url_str.into(),
                })
            }
        }
    }

    pub fn download(&self, dest: &Path) -> Result<()> {
        if !dest.exists() {
            fs::create_dir_all(dest)?;
        }
        if !dest.is_dir() {
            bail!(
                "Download destination must be a directory: {}",
                dest.display()
            );
        }
        match self {
            Resource::Svn { url, .. } => Command::new("svn")
                .args(&["co", url.as_str(), "-r", "HEAD"])
                .arg(dest)
                .check_run()?,
            Resource::Git { url, branch } => {
                info!("Git clone {}", url);
                let mut git = Command::new("git");
                git.args(&["clone", url.as_str(), "--depth", "1"]).arg(dest);
                if let Some(branch) = branch {
                    git.args(&["-b", branch]);
                }
                git.check_run()?;
            }
            Resource::Tar { url } => {
                let path = download_file(url, &dest)?;
                Command::new("tar")
                    .arg("xf")
                    .arg(path.file_name().unwrap())
                    .current_dir(dest)
                    .check_run()?;
            }
        }
        Ok(())
    }

    pub fn update(&self, dest: &Path) -> Result<()> {
        match self {
            Resource::Svn { .. } => Command::new("svn")
                .arg("update")
                .current_dir(dest)
                .check_run()?,
            Resource::Git { .. } => Command::new("git")
                .arg("pull")
                .current_dir(dest)
                .check_run()?,
            Resource::Tar { .. } => {}
        }
        Ok(())
    }
}

fn get_filename_from_url(url_str: &str) -> Result<String> {
    let url = ::url::Url::parse(url_str)?;
    let seg = url.path_segments().ok_or(err_msg("URL parse failed"))?;
    let filename = seg.last().ok_or(err_msg("URL is invalid"))?;
    Ok(filename.to_string())
}

fn download_file(url: &str, temp: &Path) -> Result<PathBuf> {
    info!("Download: {}", url);
    let mut req = reqwest::get(url)?;
    let out = if temp.is_dir() {
        let name = get_filename_from_url(url)?;
        temp.join(name)
    } else {
        temp.into()
    };
    let mut f = fs::File::create(&out)?;
    req.copy_to(&mut f)?;
    f.sync_all()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn parse_tar_url() -> Result<()> {
        let tar_url = "http://releases.llvm.org/6.0.1/llvm-6.0.1.src.tar.xz";
        match Resource::from_url(tar_url)? {
            Resource::Tar { url } => {
                assert_eq!(url, tar_url);
            }
            _ => unreachable!("Invalid detection"),
        }
        Ok(())
    }

    #[test]
    fn parse_svn_url() -> Result<()> {
        let svn_url = "http://llvm.org/svn/llvm-project/llvm/trunk";
        match Resource::from_url(svn_url)? {
            Resource::Svn { url } => {
                assert_eq!(url, svn_url);
            }
            _ => unreachable!("Invalid detection"),
        }
        Ok(())
    }

    #[test]
    fn parse_git_url() -> Result<()> {
        let git_url = "http://github.com/termoshtt/llvmenv";
        match Resource::from_url(git_url)? {
            Resource::Git { url, branch: _ } => {
                assert_eq!(url, git_url);
            }
            _ => unreachable!("Invalid detection"),
        }
        Ok(())
    }

    // Test donwloading this repo
    #[test]
    fn test_git_donwload() -> Result<()> {
        let git = Resource::Git {
            url: "http://github.com/termoshtt/llvmenv".into(),
            branch: None,
        };
        let tmp_dir = TempDir::new("git_download_test")?;
        git.download(tmp_dir.path())?;
        let cargo_toml = tmp_dir.path().join("Cargo.toml");
        assert!(cargo_toml.exists());
        Ok(())
    }

    #[test]
    fn test_tar_download() -> Result<()> {
        let tar = Resource::Tar {
            url: "https://github.com/termoshtt/llvmenv/archive/0.1.10.tar.gz".into(),
        };
        let tmp_dir = TempDir::new("tar_download_test")?;
        tar.download(tmp_dir.path())?;
        let cargo_toml = tmp_dir.path().join("Cargo.toml");
        assert!(cargo_toml.exists());
        Ok(())
    }

    #[test]
    fn test_get_filename_from_url() {
        let url = "http://releases.llvm.org/6.0.1/llvm-6.0.1.src.tar.xz";
        assert_eq!(get_filename_from_url(url).unwrap(), "llvm-6.0.1.src.tar.xz");
    }

}
