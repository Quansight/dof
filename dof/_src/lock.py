import asyncio
import yaml
from rattler import solve, VirtualPackage

from dof._src.models.environment import CondaEnvironmentSpec


def lock_environment(path: str):
    lock_spec =  _parse_environment_file(path)

    solution_packages = asyncio.run(
        _solve_environment(lock_spec=lock_spec)
    )
    print(solution_packages)


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