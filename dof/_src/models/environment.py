import json
import pathlib
from typing import Any

from pydantic import BaseModel, Field, field_serializer, field_validator
from rattler import PrefixRecord, RepoDataRecord, PackageRecord

from dof._src.models import package


class CondaEnvironmentSpec(BaseModel):
    """Input conda environment.yaml spec"""

    name: str | None
    channels: list[str]
    dependencies: list[str]
    variables: dict[str, str] | None = Field(default={})


class EnvironmentMetadata(BaseModel):
    """Metadata for an environment"""

    spec_version: str = "0.0.1"
    platform: str
    build_hash: str
    channels: list[str]
    conda_settings: dict[str, Any] | None = Field(default={})


class EnvironmentSpec(BaseModel):
    """Specifies a locked environment

    A lock exists for each platform. So to fully represent a locked environment
    across multiple platforms you will need multiple EnvironmentSpecs.
    """

    metadata: EnvironmentMetadata
    packages: list[package.Package]
    env_vars: dict[str, str] | None = None

class EnvironmentCheckpoint(BaseModel):
    """An environment at a point in time

    Only applys to a particular environment spec
    """

    environment: EnvironmentSpec
    timestamp: str
    uuid: str
    tags: list[str]
