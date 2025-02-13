import asyncio
import yaml

from typing import List

from rattler import solve, Platform

from dof._src.models.environment import CondaEnvironmentSpec, EnvironmentSpec, EnvironmentMetadata
from dof._src.models.package import UrlCondaPackage
from dof._src.utils import hash_string


# TODO: don't use this
def lock_environment(path: str, target_platform: str | None = None) -> EnvironmentSpec:
    lock_spec =  _parse_environment_file(path)

    if target_platform is None:
        target_platform = Platform.current()

    solution_packages = asyncio.run(
        _solve_environment(lock_spec=lock_spec, platforms=[target_platform])
    )

    url_packages = []
    for pkg in solution_packages:
        url_packages.append(UrlCondaPackage(url = pkg.url))

    env_metadata = EnvironmentMetadata(
        platform = str(target_platform),
        channels = lock_spec.channels,
        build_hash = hash_string(str(url_packages)),
    )

    env_spec = EnvironmentSpec(
        metadata = env_metadata,
        packages = url_packages,
    )

    return env_spec


def _parse_environment_file(path: str) -> CondaEnvironmentSpec:
    with open(path, 'r') as file:
        raw_env_spec = yaml.safe_load(file)
    
    env_spec = CondaEnvironmentSpec.parse_obj(raw_env_spec)
    return env_spec


async def _solve_environment(lock_spec: CondaEnvironmentSpec, platforms: List[Platform]):
    # rattler solve works multiplatform and is super fast
    solved_records = await solve(
        channels=lock_spec.channels,
        specs=lock_spec.dependencies,
        platforms=platforms,
    )
    return solved_records