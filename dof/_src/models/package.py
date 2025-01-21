from typing import Dict, Union

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
    url: str


class UrlPackage(BaseModel):
    url: str


Package = Union[CondaPackage, PipPackage, UrlPackage]
