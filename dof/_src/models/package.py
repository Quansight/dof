from typing import Union, Optional
from rattler import RepoDataRecord, PackageRecord
from pydantic import BaseModel


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
        

    def __str__(self):
        return f"conda: {self.name} - {self.version}"
    
    def __eq__(self, other):
        if isinstance(other, CondaPackage):
            return self.url == other.url
        return False


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


class UrlPackage(BaseModel):
    url: str

    def __str__(self):
        package = self.url.split("/")[-1]
        version = package.split("-")[-2]
        name = "-".join(package.split("-")[:-2])
        return f"url: {name} - {version}"


Package = Union[CondaPackage, PipPackage, UrlPackage]
