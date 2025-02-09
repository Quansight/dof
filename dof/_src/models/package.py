import datetime
from typing import Any

from pydantic import BaseModel


class CondaPackage(BaseModel):
    name: str
    version: str
    build: str
    platform: str
    conda_channel: str
    hash: dict[str, str]


class PipPackage(BaseModel):
    name: str
    version: str
    build: str
    url: str | None = None

    def __str__(self):
        return f"pip: {self.name} - {self.version}"


class UrlPackage(BaseModel):
    url: str

    def __str__(self):
        package = self.url.split("/")[-1]
        version = package.split("-")[-2]
        name = "-".join(package.split("-")[:-2])
        return f"conda: {name} - {version}"


# Package = Union[CondaPackage, PipPackage, UrlPackage]


class Package(BaseModel):
    arch: str | None
    build: str
    build_number: int
    channel: str
    constrains: list[str]
    depends: list[str]
    extracted_package_dir: str
    # features: str
    file_name: str
    files: list[str]
    # legacy_bz2_md5: str
    # legacy_bz2_size: str
    license: str
    license_family: str | None
    # matches: str
    md5: bytes
    name: str
    # noarch: str
    package_tarball_full_path: str | None
    # paths_data: str
    platform: str | None
    python_site_packages_path: str | None
    requested_spec: str | None
    sha256: bytes
    size: int
    subdir: str
    timestamp: datetime.datetime
    track_features: list[str]
    url: str
    version: str
