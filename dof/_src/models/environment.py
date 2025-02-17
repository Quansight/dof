from typing import Dict, List, Optional, Any

from pydantic import BaseModel, Field

from dof._src.models import package
from dof._src.models import conda_lock


class CondaEnvironmentSpec(BaseModel):
    """Input conda environment.yaml spec"""
    name: Optional[str]
    channels: List[str]
    dependencies: List[str]
    variables: Optional[Dict[str, str]] = Field(default={})


class EnvironmentMetadata(BaseModel):
    """Metadata for an environment"""
    spec_version: str = "0.0.1"
    platform: str
    build_hash: str
    channels: List[str]
    conda_settings: Optional[Dict[str, Any]] = Field(default={})


class EnvironmentSpec(BaseModel):
    """Specifies a locked environment
    
    A lock exists for each platform. So to fully represent a locked environment
    across multiple platforms you will need multiple EnvironmentSpecs.
    """
    metadata: EnvironmentMetadata
    packages: List[package.Package]
    env_vars: Optional[Dict[str, str]] = None

    def to_conda_lock_file(self) -> str:
        channels = [{"url": chn} for chn in self.metadata.channels]
        packages = [
            pkg.to_conda_lock_package(self.metadata.platform)
            for pkg in self.packages
        ]
        return conda_lock.CondaLockFile(
            metadata = conda_lock.CondaLockMetadata(
                channels = channels,
                platforms = [self.metadata.platform],
                sources = ["prefixdata"],
                content_hash = {}
            ),
            package = packages,
            version = 1,
        )
    
    def to_pixi_lock_file(self) -> str:
        return "pixi lockfile"


class EnvironmentCheckpoint(BaseModel):
    """An environment at a point in time
    
    Only applys to a particular environment spec
    """
    environment: EnvironmentSpec
    timestamp: str
    uuid: str
    tags: List[str]
