from typing import List
import datetime

from conda.core.prefix_data import PrefixData
from rattler import Platform

from dof._src.models import package, environment
from dof._src.utils import hash_string
from dof._src.data.local import LocalData


class Checkpoint():
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

        env_metadata = environment.EnvironmentMetadata(
            platform = str(Platform.current()),
            channels = channels,
            build_hash = hash_string(str(packages)),
        )
        env_spec = environment.EnvironmentSpec(
            packages=packages,
            metadata=env_metadata,
        )
        env_checkpoint= environment.EnvironmentCheckpoint(
            environment=env_spec,
            timestamp=str(datetime.datetime.now(datetime.UTC)),
            uuid=uuid,
            tags=tags,
        )
        return cls(env_checkpoint=env_checkpoint, prefix=prefix)
    
    def __init__(self, env_checkpoint: environment.EnvironmentCheckpoint, prefix: str):
        self.env_checkpoint = env_checkpoint
        self.prefix = prefix
        # TODO: this can be swapped out for a different data 
        # dir type, eg to support remote data dirs
        self.data_dir = LocalData()

    def save(self):
        self.data_dir.save_environment_checkpoint(self.env_checkpoint, self.prefix)

    def diff(self, revision: str):
        pass