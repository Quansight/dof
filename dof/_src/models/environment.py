from typing import Dict, List, Optional, Union, Any

from pydantic import BaseModel

from dof._src.models.package import Package


class EnvironmentMetadata(BaseModel):
    platform: str
    build_hash: str
    channels: List[str]
    conda_settings: Dict[str, Any]


class EnvironmentSpec(BaseModel):
    metadata: EnvironmentMetadata
    packages: List[Package]
    env_vars: Optional[Dict[str, str]] = None
