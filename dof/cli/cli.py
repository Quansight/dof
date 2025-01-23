from typing import Optional
import yaml

import typer

from dof._src.lock import lock_environment


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
def checkpoint():
    """Create a lockfile for the current env and set a checkpoint"""
    print("todo")
