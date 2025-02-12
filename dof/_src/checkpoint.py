from __future__ import annotations

import asyncio
import datetime
from collections import defaultdict

import yaml
from conda.core.prefix_data import PrefixData
from rattler import Platform, install, LockFile

from dof._src.data.local import LocalData
from dof._src.models import environment, package
from dof._src.utils import hash_string


class Checkpoint:
    @classmethod
    def from_prefix(cls, prefix: str, uuid: str, directory: str, tags: list[str] | None = None) -> Checkpoint:
        if tags is None:
            tags = []

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
            dofspec=environment.Dofspec.generate_project_dofspec(directory),
        )
        return cls(env_checkpoint=env_checkpoint, prefix=prefix)

    @classmethod
    def from_uuid(cls, prefix: str, uuid: str) -> Checkpoint:
        data_dir = LocalData()
        env_checkpoint = data_dir.get_environment_checkpoint(prefix, uuid)
        return cls(env_checkpoint=env_checkpoint, prefix=prefix)

    @classmethod
    def from_checkpoint_dict(cls, checkpoint_data: dict, prefix: str) -> Checkpoint:
        env_checkpoint = environment.EnvironmentCheckpoint.model_validate(checkpoint_data)
        return cls(env_checkpoint=env_checkpoint, prefix=prefix)

    def __init__(self, env_checkpoint: environment.EnvironmentCheckpoint, prefix: str):
        self.env_checkpoint = env_checkpoint
        self.prefix = prefix
        # TODO: this can be swapped out for a different data
        # dir type, eg to support remote data dirs
        self.data_dir = LocalData()

    def save(self) -> None:
        self.data_dir.save_environment_checkpoint(self.env_checkpoint, self.prefix)

    def diff(self, revision: str) -> tuple[list[str], list[str]]:
        target_checkpoint = self.data_dir.get_environment_checkpoint(self.prefix, uuid=revision)
        target_packages = target_checkpoint.environment.packages
        current_packages = self.env_checkpoint.environment.packages

        packages_in_current_not_in_target = [item for item in current_packages if item not in target_packages]
        packages_in_target_not_in_current = [item for item in target_packages if item not in current_packages]
        return packages_in_current_not_in_target, packages_in_target_not_in_current

    def list_packages(self) -> list[str]:
        return self.env_checkpoint.environment.packages

    def install(self) -> None:
        lock = yaml.safe_load(self.env_checkpoint.dofspec.lock)

        records = []
        breakpoint()
        for env_name, env in lock.environments():
            for platform, packages in env.packages_by_platform():
                print(packages)
        records = []
        asyncio.run(
            install(
                records=records,
                target_prefix=self.prefix,
            )
        )
