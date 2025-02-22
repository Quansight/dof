use std::error::Error;
use url::Url;

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
