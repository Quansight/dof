import asyncio

import yaml
from rattler import Platform, RepoDataRecord, VirtualPackage, solve

from dof._src.models.environment import (
    CondaEnvironmentSpec,
    EnvironmentMetadata,
    EnvironmentSpec,
)
from dof._src.models.package import UrlPackage
from dof._src.utils import hash_string


def lock_environment(path: str, target_platform: str | None = None) -> EnvironmentSpec:
    lock_spec =  _parse_environment_file(path)

    if target_platform is None:
        target_platform = Platform.current()

    solution_packages = asyncio.run(
        _solve_environment(lock_spec=lock_spec, platforms=[target_platform])
    )

    url_packages = []
    for pkg in solution_packages:
        url_packages.append(UrlPackage(url = pkg.url))

    env_metadata = EnvironmentMetadata(
        platform = str(target_platform),
        channels = lock_spec.channels,
        build_hash = hash_string(str(url_packages)),
    )

    return EnvironmentSpec(
        metadata = env_metadata,
        packages = url_packages,
        solved_packages = solution_packages
    )


def _parse_environment_file(path: str) -> CondaEnvironmentSpec:
    with open(path) as file:
        raw_env_spec = yaml.safe_load(file)

    return CondaEnvironmentSpec.parse_obj(raw_env_spec)


async def _solve_environment(
    lock_spec: CondaEnvironmentSpec,
    platforms: list[Platform],
) -> list[RepoDataRecord]:
    # rattler solve works multiplatform and is super fast
    return await solve(
        channels=lock_spec.channels,
        specs=lock_spec.dependencies,
        platforms=platforms,
        virtual_package=VirtualPackage.detect()
    )
