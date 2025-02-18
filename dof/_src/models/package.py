from typing import Union, Optional
from rattler import RepoDataRecord, PackageRecord
from pydantic import BaseModel

from dof._src.models import conda_lock

class CondaPackage(BaseModel):
    name: str
    version: str
    build: str
    build_number: int
    subdir: str
    # must be the full url of the channel, eg. 'https://conda.anaconda.org/conda-forge/win-64'
    conda_channel: str
    arch: str
    platform: str
    url: str
    sha256: Optional[str] = None
    md5: Optional[str] = None

    def __str__(self):
        return f"conda: {self.name} - {self.version}"
    
    def __eq__(self, other):
        if isinstance(other, CondaPackage):
            return self.url == other.url
        return False

    def to_repodata_record(self):
        """Converts a url package into a rattler compatible repodata record."""
        pkg_record = PackageRecord(
             name=self.name, version=self.version, build=self.build,
             build_number=self.build_number, subdir=self.subdir, arch=None,
             platform=None
        )
        return RepoDataRecord(
            package_record=pkg_record,
            file_name=self.url.split("/")[-1],
            channel=self.conda_channel,
            url=self.url
        )
    
    def to_conda_lock_package(self, platform):
        return conda_lock.CondaLockPackage(
            category = "main",
            name = self.name,
            version = self.version,
            dependencies = {},
            hash = {
                "sha256": self.sha256,
                "md5": self.md5,
            },
            manager = "conda",
            optional = False,
            platform = platform,
            url = self.url,
        )


class PipPackage(BaseModel):
    name: str
    version: str
    build: str
    url: Optional[str] = None
    
    def __str__(self):
        return f"pip: {self.name} - {self.version}"
    
    def __eq__(self, other):
        if isinstance(other, PipPackage):
            return self.name == other.name and self.version == other.version and self.build == other.build
        return False
    
    def to_repodata_record(self):
        """Converts a url package into a rattler compatible repodata record."""
        # no-op
        pass

    def to_conda_lock_package(self, platform):
        return conda_lock.CondaLockPackage(
            category = "main",
            name = self.name,
            version = self.version,
            manager = "pip",
            optional = False,
            platform = platform,
            dependencies = {},
            hash = {},
            url = self.url,
        )

class UrlPackage(BaseModel):
    url: str

    def manager(self) -> str:
        """Returns the package manager for this package."""
        return "None" 

    def __str__(self):
        package = self.url.split("/")[-1]
        version = package.split("-")[-2]
        name = "-".join(package.split("-")[:-2])
        return f"url: {name} - {version}"


Package = Union[CondaPackage, PipPackage, UrlPackage]
