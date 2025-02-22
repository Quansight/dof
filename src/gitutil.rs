use std::path::PathBuf;
use std::error::Error;
use std::str::FromStr;
use url::Url;

use crate::RepositoryUrl;

pub struct LockedGitUrl(Url);

impl LockedGitUrl {
    /// Creates a new [`LockedGitUrl`] from a [`Url`].
    pub fn new(url: Url) -> Self {
        Self(url)
    }

    /// Returns true if the given URL is a locked git URL.
    /// This is used to differentiate between a regular Url and a [`LockedGitUrl`]
    /// that starts with `git+`.
    pub fn is_locked_git_url(locked_url: &Url) -> bool {
        locked_url.scheme().starts_with("git+")
    }

    /// Parses a locked git URL from a string.
    pub fn parse(url: &str) -> Result<Self, Box<dyn Error>> {
        let url = Url::parse(url)?;
        Ok(Self(url))
    }

    /// Converts this [`LockedGitUrl`] into a [`Url`].
    pub fn to_url(&self) -> Url {
        self.0.clone()
    }
}

impl From<LockedGitUrl> for Url {
    fn from(value: LockedGitUrl) -> Self {
        value.0
    }
}

/// Convert a locked git url into a parsed git url
/// [`LockedGitUrl`] is always recorded in the lock file and looks like this:
/// <git+https://git.example.com/MyProject.git?tag=v1.0&subdirectory=pkg_dir#1c4b2c7864a60ea169e091901fcde63a8d6fbfdc>
///
/// [`uv_pypi_types::ParsedGitUrl`] looks like this:
/// <git+https://git.example.com/MyProject.git@v1.0#subdirectory=pkg_dir>
///
/// So we need to convert the locked git url into a parsed git url.
/// which is used in the uv crate.
pub fn to_parsed_git_url(
    locked_git_url: &LockedGitUrl,
) -> Result<uv_pypi_types::ParsedGitUrl, Box<dyn Error>> {
    let git_source = PinnedGitCheckout::from_locked_url(locked_git_url)?;
    // Construct manually [`ParsedGitUrl`] from locked url.
    let parsed_git_url = uv_pypi_types::ParsedGitUrl::from_source(
        RepositoryUrl::new(&locked_git_url.to_url()).into(),
        uv_git::GitReference::from_rev(git_source.reference),
        Some(uv_git::GitOid::from_str(&git_source.commit)?),
        git_source.subdirectory.map(|s| PathBuf::from(s.as_str())),
    );

    Ok(parsed_git_url)
}


/// A pinned version of a git checkout.
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct PinnedGitCheckout {
    /// The commit hash of the git checkout.
    pub commit: String,
    /// The subdirectory of the git checkout.
    pub subdirectory: Option<String>,
    /// The reference of the git checkout.
    pub reference: String,
}

impl PinnedGitCheckout {
    /// Creates a new pinned git checkout.
    pub fn new(commit: uv_git::GitOid, subdirectory: Option<String>, reference: String) -> Self {
        Self {
            commit: commit.to_string(),
            subdirectory,
            reference,
        }
    }

    /// Extracts a pinned git checkout from the query pairs and the hash
    /// fragment in the given URL.
    pub fn from_locked_url(locked_url: &LockedGitUrl) -> Result<PinnedGitCheckout, Box<dyn Error>> {
        let url = &locked_url.to_url();
        let mut reference = None;
        let mut subdirectory = None;

        for (key, val) in url.query_pairs() {
            match &*key {
                "tag" => {
                    if reference.replace(val.into_owned()).is_some() {
                        return Err("multiple tags in URL".into());
                    }
                }
                "branch" => {
                    if reference.replace(val.into_owned()).is_some() {
                        return Err("multiple branches in URL".into());
                    }
                }
                "rev" => {
                    if reference.replace(val.into_owned()).is_some() {
                        return Err("multiple revs in URL".into());
                    }
                }
                // If the URL points to a subdirectory, extract it, as in (git):
                //   `git+https://git.example.com/MyProject.git@v1.0#subdirectory=pkg_dir`
                //   `git+https://git.example.com/MyProject.git@v1.0#egg=pkg&subdirectory=pkg_dir`
                "subdirectory" => {
                    if subdirectory.replace(val.into_owned()).is_some() {
                        return Err("multiple subdirectories in URL".into());
                    }
                }
                _ => continue,
            };
        }

        // set the default reference if none is provided.
        if reference.is_none() {
            reference.replace("".into());
        }

        let commit = uv_git::GitOid::from_str(
            url.fragment().ok_or("missing sha".to_string())?
        )?.to_string();

        Ok(PinnedGitCheckout {
            commit,
            subdirectory,
            reference: reference.expect("reference should be set"),
        })
    }
}
