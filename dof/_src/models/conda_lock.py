from typing import Any, Optional

from pydantic import BaseModel


class CondaLockMetadata(BaseModel):
    channels: list[dict[str, Any]]
    content_hash: dict[str, str]
    platforms: list[str]
    sources: list[str]

class CondaLockPackage(BaseModel):
    category: str
    name: str
    version: str
    dependencies: dict[str, str]
    hash: dict[str, str]
    manager: str
    optional: bool
    platform: str
    url: Optional[str] = None

class CondaLockFile(BaseModel):
    metadata: CondaLockMetadata
    package: list[CondaLockPackage]
    version: int