import os

import rich
import typer
from rich.table import Table

from dof._src.checkpoint import Checkpoint
from dof._src.data.local import LocalData
from dof._src.utils import short_uuid

checkpoint_command = typer.Typer(
    add_completion=False,
    no_args_is_help=True,
    rich_markup_mode="rich",
    context_settings={"help_option_names": ["-h", "--help"]},
)


@checkpoint_command.command()
def save(
    ctx: typer.Context,
    tags: list[str] = typer.Option(
        None,
        help="tags for the checkpoint"
    ),
    directory: str | None = typer.Option(
        None,
        help="directory in which the lockfile for the given env lives"
    )
):
    """Create a lockfile for the current env and set a checkpoint.

    Assumes that the user is currently in a conda environment
    """
    prefix = os.environ.get("CONDA_PREFIX")
    env_uuid = short_uuid()
    if tags is None:
        tags = [env_uuid]

    chck = Checkpoint.from_prefix(
        prefix=prefix,
        tags=tags,
        uuid=env_uuid,
        directory=directory if directory is not None else "./",
    )
    chck.save()


@checkpoint_command.command()
def delete(
    ctx: typer.Context,
    rev: str = typer.Option(
        help="uuid of the revision to delete"
    ),
):
    """Delete a previous revision of the environment"""
    prefix = os.environ.get("CONDA_PREFIX")
    data = LocalData()
    data.delete_environment_checkpoint(prefix=prefix, uuid=rev)


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
    _ctx: typer.Context,
    rev: str = typer.Option(
        help="uuid of the revision to install"
    ),
) -> None:
    """Install a previous revision of the environment."""
    prefix = os.environ.get("CONDA_PREFIX")
    Checkpoint.from_uuid(prefix, rev).install()


@checkpoint_command.command()
def diff(
    ctx: typer.Context,
    rev: str = typer.Option(
        help="uuid of the revision to diff against"
    ),
):
    """Generate a diff of the current environment to the specified revision"""
    prefix = os.environ.get("CONDA_PREFIX")
    env_uuid = short_uuid()
    chck = Checkpoint.from_prefix(prefix=prefix, uuid=env_uuid)
    packages_in_current_not_in_target, packages_in_target_not_in_current = chck.diff(rev)

    print(f"diff with rev {rev}")
    for pkg in packages_in_current_not_in_target:
        print(f"+ {pkg}")
    for pkg in packages_in_target_not_in_current:
        print(f"- {pkg}")

@checkpoint_command.command()
def show(
    ctx: typer.Context,
    rev: str = typer.Option(
        help="uuid of the revision to list packages for"
    ),
):
    """Generate a list packages in an environment revision"""
    prefix = os.environ.get("CONDA_PREFIX")
    chck = Checkpoint.from_uuid(prefix=prefix, uuid=rev)
    for pkg in chck.list_packages():
        print(pkg)
