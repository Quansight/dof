import yaml
import os
import typer
from typing import List
import uuid

from dof._src.lock import lock_environment
from dof._src.models.environment import EnvironmentCheckpoint
from dof._src.data.local import LocalData


app = typer.Typer()


@app.command()
def install(
    path: str = typer.Option(
        help="path to lockfile"
    ),
):
    """Install a lockfile
    """
    print("not really installing")


@app.command()
def lock(
    env_file: str = typer.Option(
        help="path to environment file"
    ),
    output: str = typer.Option(
        None,
        help="path to output lockfile"
    ),
):
    """Generate a lockfile"""
    solved_env = lock_environment(path=env_file)
    
    # If no output is specified dump yaml output to stdout
    if output is None:
        print(yaml.dump(solved_env.model_dump()))
    else:
        with open(output, "w+") as env_file:
            yaml.dump(solved_env.model_dump(), env_file)


@app.command()
def checkpoint(
    tags: List[str] = typer.Option(
        None,
        help="path to output lockfile"
    ),
):
    """Create a lockfile for the current env and set a checkpoint.
    
    Assumes that the user is currently in a conda environment
    """
    prefix = os.environ.get("CONDA_PREFIX")
    env_uuid = uuid.uuid4().hex
    if tags is None:
        tags = [env_uuid]

    chck = EnvironmentCheckpoint.from_prefix(prefix=prefix, tags=tags, uuid=env_uuid)
    data = LocalData()
    data.save_environment_checkpoint(chck, prefix)
