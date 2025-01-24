import yaml
import os
import typer
from typing import List

from dof._src.lock import lock_environment
from dof.cli.checkpoint import checkpoint_command

app = typer.Typer(
    add_completion=False,
    no_args_is_help=True,
    rich_markup_mode="rich",
    context_settings={"help_option_names": ["-h", "--help"]},
)

app.add_typer(
    checkpoint_command,
    name="checkpoint",
    help="create and manage checkpoints",
    rich_help_panel="Checkpoint",
)

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
