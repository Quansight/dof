# TODO: rename this to `data_dir` and move up one module - this doesn't need it's whole own module
from pathlib import Path
from typing import List
import os
import yaml
import base64

from dof._src.models import environment
from dof._src.utils import ensure_dir

def default_data_dir() -> Path:
    dof_dir = os.environ.get("DOF_DIR", None)
    if dof_dir is None:
        dof_dir = Path.home() / ".dof"
    else:
        dof_dir = Path(dof_dir)
    
    return dof_dir / "data"

class LocalData:
    def __init__(self, data_dir: str | None = None):
        self.data_dir = data_dir
        if self.data_dir is None:
            self.data_dir = str(default_data_dir())
        
        ensure_dir(self.data_dir)

    def _get_env_dir(self, prefix: str):
        # Not stoked on this approach. But provides a mapping
        # between a prefix path and folder in a way that doesn't
        # get too long and remains unique
        name = prefix.replace("/", "-")
        return f"{self.data_dir}/{name}"

    def delete_environment_checkpoint(self, prefix: str, uuid: str):
        target_dir = self._get_env_dir(prefix)
        target_file = f"{target_dir}/{uuid}"
        if os.path.exists(target_file):
            os.remove(target_file)

    def save_environment_checkpoint(self, checkpoint: environment.EnvironmentCheckpoint, prefix: str):
        target_dir = self._get_env_dir(prefix)
        ensure_dir(target_dir)
        
        target_file = f"{target_dir}/{checkpoint.uuid}"
        with open(target_file, "w+") as file:
            yaml.dump(checkpoint.model_dump(), file)
    
    def get_environment_checkpoints(self, prefix: str) -> List[environment.EnvironmentCheckpoint]:
        target_dir = self._get_env_dir(prefix)
        if not os.path.exists(target_dir):
            return []
        files = os.listdir(target_dir)

        checkpoints = []
        for file in files:
            with open(os.path.join(target_dir, file), 'r') as file:
                contents = yaml.safe_load(file)
            
            checkpoints.append(environment.EnvironmentCheckpoint.parse_obj(contents))
        
        return checkpoints
    
    def get_environment_checkpoint(self, prefix: str, uuid: str) -> environment.EnvironmentCheckpoint:
        target_dir = self._get_env_dir(prefix)
        target_file = f"{target_dir}/{uuid}"
        if not os.path.exists(target_file):
            return None
        
        with open(target_file, 'r') as file:
            contents = yaml.safe_load(file)
            return environment.EnvironmentCheckpoint.parse_obj(contents)
