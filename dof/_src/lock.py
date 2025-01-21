import asyncio
import yaml
from rattler import solve, VirtualPackage

from dof._src.models.environment import CondaEnvironmentSpec, EnvironmentSpec, EnvironmentMetadata
from dof._src.models.package import UrlPackage


def lock_environment(path: str) -> EnvironmentSpec:
    lock_spec =  _parse_environment_file(path)

    solution_packages = asyncio.run(
        _solve_environment(lock_spec=lock_spec)
    )

    url_packages = []
    for pkg in solution_packages:
        url_packages.append(UrlPackage(url = pkg.url))

    # TODO: fill in these values properly
    env_metadata = EnvironmentMetadata(
        platform = "linux-64",
        channels = lock_spec.channels,
        env_version = 1,
        build_hash = "12"
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


async def _solve_environment(lock_spec: CondaEnvironmentSpec):
    solved_records = await solve(
        # Channels to use for solving
        channels=lock_spec.channels,
        # The specs to solve for
        specs=lock_spec.dependencies,
    )
    return solved_records