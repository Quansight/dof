from typing import Dict, List, Optional, Any

from pydantic import BaseModel, Field
from pydantic_yaml import parse_yaml_raw_as, to_yaml_str

from dof._src.models.package import Package


class CondaEnvironmentSpec(BaseModel):
    name: Optional[str]
    channels: List[str]
    dependencies: List[str]
    variables: Optional[Dict[str, str]] = Field(default={})


class EnvironmentMetadata(BaseModel):
    platform: str
    build_hash: str
    channels: List[str]
    conda_settings: Dict[str, Any]


class EnvironmentSpec(BaseModel):
    metadata: EnvironmentMetadata
    packages: List[Package]
    env_vars: Optional[Dict[str, str]] = None
