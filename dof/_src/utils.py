import hashlib
import uuid
from pathlib import Path


def hash_string(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def get_name_from_prefix(prefix: str) -> str:
    """This function assumes an environment name is the last word in a conda prefix"""
    return prefix.split("/")[-1]


def ensure_dir(s: str) -> None:
    """Recursively create a directory if it does not exist"""
    path = Path(s)
    path.mkdir(parents=True, exist_ok=True)


def get_project_root(directory: str | Path, root_path: Path = Path(".git/")) -> Path | None:
    """Identify the project root directory: the one that contains `root_path`.

    Parameters
    ----------
    directory : str | Path
        Directory which is a child of the root directory
    root_path : Path
        Path which identifies the root of the project. Usually this is
        "pyproject.toml" or ".git/"

    Returns
    -------
    Path | None
        Path to the project root, or None if a root cannot be found
    """
    directory = Path(directory).resolve()

    filesystem_root = directory.parents[-1]
    while directory != filesystem_root:
        for item in directory.iterdir():
            if root_path.name == item.name:
                return directory

        directory = directory.parent.resolve()

    if root_path in directory.iterdir():
        return root_path

    return None


def short_uuid() -> str:
    return uuid.uuid4().hex[:8]
