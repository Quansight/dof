from typing import Dict, Union, Optional

from pydantic import BaseModel


class CondaPackage(BaseModel):
    name: str
    version: str
    build: str
    platform: str
    conda_channel: str
    hash: Dict[str, str]


class PipPackage(BaseModel):
    name: str
    version: str
    build: str
    url: Optional[str] = None

    def __str__(self):
        return f"pip: {self.name} - {self.version}"


class UrlPackage(BaseModel):
    url: str

    def __str__(self):
        package = self.url.split("/")[-1]
        version = package.split("-")[-2]
        name = "-".join(package.split("-")[:-2])
        return f"conda: {name} - {version}"


Package = Union[CondaPackage, PipPackage, UrlPackage]
