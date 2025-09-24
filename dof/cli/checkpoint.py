import os
import typer
from typing import List
from typing_extensions import Annotated
import asyncio

from rich.table import Table
import rich

from dof._src.checkpoint import Checkpoint
from dof._src.data.local import LocalData
from dof._src.utils import short_uuid
from dof._src.constants import SupportedExportFormats


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
    prefix: str = typer.Option(
        None,
        help="prefix to save"
    ),
):
    """Create a checkpoint for the current state of an environment.
    
    If no prefix is specified, assumes the current conda environment.
    """
    if prefix is None:
        prefix = os.environ.get("CONDA_PREFIX")
    else:
        prefix = os.path.abspath(prefix)
    
    env_uuid = short_uuid()
    if tags is None:
        tags = [env_uuid]

    chck = Checkpoint.from_prefix(prefix=prefix, tags=tags, uuid=env_uuid)
    chck.save()


@checkpoint_command.command()
def delete(
    ctx: typer.Context,
    rev: str = typer.Option(
        help="uuid of the revision to delete"
    ),
     prefix: str = typer.Option(
        None,
        help="prefix to save"
    ),
):
    """Delete a previous revision of the environment"""
    if prefix is None:
        prefix = os.environ.get("CONDA_PREFIX")
    else:
        prefix = os.path.abspath(prefix)
    data = LocalData()
    data.delete_environment_checkpoint(prefix=prefix, uuid=rev)


@checkpoint_command.command()
def list(
    ctx: typer.Context,
    prefix: str = typer.Option(
        None,
        help="prefix to save"
    ),
):
    """List all checkpoints for the current environment"""
    data = LocalData()
    if prefix is None:
        prefix = os.environ.get("CONDA_PREFIX")
    else:
        prefix = os.path.abspath(prefix)
    
    checkpoints = data.get_environment_checkpoints(prefix=prefix)
    checkpoints.sort(key=lambda x: x.timestamp, reverse=True)

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
    rev: str = typer.Option(
        help="uuid of the revision to install"
    ),
    prefix: str = typer.Option(
        None,
        help="prefix to install"
    ),
):
    """Install a previous revision of the environment"""
    if prefix is None:
        prefix = os.environ.get("CONDA_PREFIX")
    else:
        prefix = os.path.abspath(prefix)
    env_uuid = short_uuid()
    chck = Checkpoint.from_prefix(prefix=prefix, uuid=env_uuid)
    packages_in_current_not_in_target, packages_in_target_not_in_current = chck.diff(rev)

    print("!!!WARNING!!! This probably won't work if you have pip packages installed in your target prefix")

    print("packages to delete")
    for pkg in packages_in_current_not_in_target:
        print(f"- {pkg}")
    print("\npackages to install")
    for pkg in packages_in_target_not_in_current:
        print(f"+ {pkg}")

    rev_checkpoint = Checkpoint.from_uuid(prefix=prefix, uuid=rev)
    asyncio.run(rev_checkpoint.install_with_rattler())


@checkpoint_command.command()
def diff(
    ctx: typer.Context,
    rev: str = typer.Option(
        help="uuid of the revision to diff against"
    ),
    prefix: str = typer.Option(
        None,
        help="prefix to diff"
    ),
):
    """Generate a diff of the current environment to the specified revision"""
    if prefix is None:
        prefix = os.environ.get("CONDA_PREFIX")
    else:
        prefix = os.path.abspath(prefix)
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
        None,
        help="uuid of the revision to list packages for"
    ),
    prefix: str = typer.Option(
        None,
        help="prefix to show"
    ),
):
    """Generate a list packages in an environment revision"""
    if prefix is None:
        prefix = os.environ.get("CONDA_PREFIX")
    else:
        prefix = os.path.abspath(prefix)

    if rev is None:
        env_uuid = short_uuid()
        chck = Checkpoint.from_prefix(prefix=prefix, uuid=env_uuid)
    else:
        chck = Checkpoint.from_uuid(prefix=prefix, uuid=rev)

    for pkg in chck.list_packages():
        print(pkg)


@checkpoint_command.command()
def export(
    ctx: typer.Context,
    rev: str = typer.Option(
        None,
        help="uuid of the revision to export"
    ),
    prefix: str = typer.Option(
        None,
        help="prefix to export"
    ),
    format: Annotated[
        SupportedExportFormats,
        typer.Option(
            default=...,
            help="format to export to"
        ),
    ] = ...
):
    """Export the revision to given format"""
    if prefix is None:
        prefix = os.environ.get("CONDA_PREFIX")
    else:
        prefix = os.path.abspath(prefix)

    if rev is None:
        env_uuid = short_uuid()
        chck = Checkpoint.from_prefix(prefix=prefix, uuid=env_uuid)
    else:
        chck = Checkpoint.from_uuid(prefix=prefix, uuid=rev)

    # TODO: export checkpoint
