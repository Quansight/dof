import hashlib
from pathlib import Path
import uuid


def hash_string(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def ensure_dir(s: str):
    """Recursively create a directory if it does not exist"""
    path = Path(s)
    path.mkdir(parents=True, exist_ok=True)


def short_uuid() -> str:
    return uuid.uuid4().hex[:8]
