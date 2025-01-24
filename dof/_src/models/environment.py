from typing import Dict, List, Optional, Any
import datetime

from pydantic import BaseModel, Field
from conda.core.prefix_data import PrefixData
from rattler import Platform


from dof._src.models import package
from dof._src.utils import hash_string


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


class EnvironmentCheckpoint(BaseModel):
    """An environment at a point in time
    
    Only applys to a particular environment spec
    """
    environment: EnvironmentSpec
    timestamp: str
    uuid: str
    tags: List[str]

    @classmethod
    def from_prefix(cls, prefix: str, uuid: str, tags: List[str] = []):
        packages = []
        channels = set()
        for prefix_record in PrefixData(prefix, pip_interop_enabled=True).iter_records_sorted():
            if prefix_record.subdir == "pypi":
                packages.append(
                    package.PipPackage(
                        name=prefix_record.name,
                        version=prefix_record.version,
                        build=prefix_record.build,
                    )
                )
            else:
                channels.add(prefix_record.channel.name)
                packages.append(
                    package.UrlPackage(url=prefix_record.url)
                )

        env_metadata = EnvironmentMetadata(
            platform = str(Platform.current()),
            channels = channels,
            build_hash = hash_string(str(packages)),
        )

        env_spec = EnvironmentSpec(
            packages=packages,
            metadata=env_metadata,
        )

        return cls(
            environment=env_spec,
            timestamp=str(datetime.datetime.now(datetime.UTC)),
            uuid=uuid,
            tags=tags,
        )
