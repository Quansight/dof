from typing import Dict, Union, Optional
from rattler import RepoDataRecord
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
        #  pr = PackageRecord(name="test-package", version="0.1", build="0", build_number=0, subdir="linux", arch="noarch", platform="linux-64")
        #  RepoDataRecord(package_record=pr, file_name="test-package-0.1-0.tar.bz2", channel="https://conda.anaconda.org/conda-forge/win-64", url="https://conda.anaconda.org/test/noarch/test-package-0.1-0.tar.bz2")
        pass

    def __str__(self):
        return f"conda: {self.name} - {self.version}"
    
    def __eq__(self, other):
        return self.url == other.url


class PipPackage(BaseModel):
    name: str
    version: str
    build: str
    url: Optional[str] = None

    def __str__(self):
        return f"pip: {self.name} - {self.version}"


class UrlCondaPackage(BaseModel):
    url: str

    def __str__(self):
        package = self.url.split("/")[-1]
        version = package.split("-")[-2]
        name = "-".join(package.split("-")[:-2])
        return f"conda: {name} - {version}"


Package = Union[CondaPackage, PipPackage, UrlCondaPackage]
