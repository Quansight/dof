from typing import Dict, List, Optional, Any

from pydantic import BaseModel, Field
from pydantic_yaml import parse_yaml_raw_as, to_yaml_str

from dof._src.models.package import Package


class CondaEnvironmentSpec(BaseModel):
    """Input conda environment.yaml spec"""
    name: Optional[str]
    channels: List[str]
    dependencies: List[str]
    variables: Optional[Dict[str, str]] = Field(default={})


class EnvironmentMetadata(BaseModel):
    """Metadata for an environment"""
    spec_version: str = "0.0.1"
    env_version: int
    platform: str
    build_hash: str
    channels: List[str]
    conda_settings: Optional[Dict[str, Any]] = Field(default={})


class EnvironmentSpec(BaseModel):
    """Specifies a locked environment"""
    metadata: EnvironmentMetadata
    packages: List[Package]
    env_vars: Optional[Dict[str, str]] = None
