import os
import typer
from typing import List
import uuid
from rich.table import Table
import rich

from dof._src.models.environment import EnvironmentCheckpoint
from dof._src.data.local import LocalData


checkpoint_command = typer.Typer(
    add_completion=False,
    no_args_is_help=True,
    rich_markup_mode="rich",
    context_settings={"help_option_names": ["-h", "--help"]},
)


@checkpoint_command.command()
def save(
    ctx: typer.Context,
    tags: List[str] = typer.Option(
        None,
        help="tags for the checkpoint"
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


@checkpoint_command.command()
def list(
    ctx: typer.Context,
):
    """List all checkpoints for the current environment"""
    data = LocalData()
    prefix = os.environ.get("CONDA_PREFIX")
    checkpoints = data.get_environment_checkpoints(prefix=prefix)

    table = Table(title="Checkpoints")
    table.add_column("uuid", justify="left", no_wrap=True)
    table.add_column("tags", justify="left", no_wrap=True)
    table.add_column("timestamp", justify="left", no_wrap=True)

    for point in checkpoints:
        table.add_row(point.uuid, str(point.tags), point.timestamp)

    rich.print(table)


@checkpoint_command.command()
def install(
    ctx: typer.Context,
    uuid: str = typer.Option(
        help="uuid of the revision to install"
    ),
):
    """Install a previous revision of the environment"""
    print("not installing")