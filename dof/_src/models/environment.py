from __future__ import annotations  # noqa: I001

from pathlib import Path
from typing import Any
from collections import defaultdict

from pydantic import BaseModel, Field, field_validator
from rattler import LockFile, Platform

from dof._src.models import package

from ..utils import get_project_root


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


class Dofspec(BaseModel):
    spec: str
    lock: str

    @classmethod
    def from_spec_and_lock(cls, specfile: str, lockfile: str) -> Dofspec:
        if specfile.is_file():
            with open(specfile) as f:
                spec = f.read()
        else:
            raise ValueError(f"No {specfile} at the project root")

        if lockfile.is_file():
            with open(lockfile) as f:
                lock = f.read()
        else:
            raise ValueError(f"No {lockfile} at the project root")

        return cls(spec=spec, lock=lock)

    @classmethod
    def generate_project_dofspec(
        cls,
        directory: str | Path,
        root_path: Path = Path(".git/"),
    ) -> Dofspec | None:
        root = get_project_root(directory, root_path)
        if not root:
            raise ValueError("Can't find project root")

        try:
            dofspec = Dofspec.from_spec_and_lock(
                specfile=root / "environment.yml",
                lockfile=root / "pixi.lock",
            )
        except ValueError:
            return None

        return dofspec


class EnvironmentCheckpoint(BaseModel):
    """An environment at a point in time

    Only applys to a particular environment spec
    """

    environment: EnvironmentSpec
    timestamp: str
    uuid: str
    tags: list[str]
    dofspec: Dofspec
