import yaml
import os
import typer
from typing_extensions import Annotated

from dof._src.lock import lock_environment
from dof._src.checkpoint import Checkpoint
from dof._src.park.park import Park
from dof.cli.checkpoint import checkpoint_command
from dof._src.conda_meta.conda_meta import CondaMeta


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


# TODO: Delete
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
def user_specs(
    rev: str = typer.Option(
        None,
        help="uuid of the revision to inspect for user_specs"
    ),
):
    """Demo command: output the list of user requested specs for a revision"""
    prefix = os.environ.get("CONDA_PREFIX")
    if rev is None:
        meta = CondaMeta(prefix=prefix)
        specs = meta.get_requested_specs()
        print("the user requested specs in this environment are:")
        # sort alphabetically for readability
        for spec in sorted(specs):
            print(f"  {spec}")
    else:
        chck = Checkpoint.from_uuid(prefix=prefix, uuid=rev)
        pkgs = chck.list_packages()
        print(f"the user requested specs rev {rev}:")
        # sort alphabetically for readability
        for spec in sorted(pkgs, key=lambda p: p.name):
            if spec.user_requested_spec is not None:
                print(f"  {spec.user_requested_spec}")


@app.command()
def push(
    target: Annotated[str, typer.Option(
        "--target", "-t",
        help="namespace/environment:tag to push to"
    )],
    rev: str = typer.Option(
        help="uuid of the revision to push"
    ),
    prefix: str = typer.Option(
        None,
        help="prefix to save"
    ),
):
    """Push a checkpoint to a target"""
    park_url = os.environ.get("PARK_URL")
    api = Park(park_url)

    namespace = target.split("/")[0]
    env_tag = target.split("/")[1]
    environment = env_tag.split(":")[0]
    tag = env_tag.split(":")[1]

    if prefix is None:
        prefix = os.environ.get("CONDA_PREFIX")
    else:
        prefix = os.path.abspath(prefix)

    chck = Checkpoint.from_uuid(prefix=prefix, uuid=rev)
    data = chck.env_checkpoint.model_dump()

    api.push(namespace, environment, tag, data)


@app.command()
def pull(
    target: Annotated[str, typer.Option(
        "--target", "-t",
        help="namespace/environment:tag to push to"
    )],
    prefix: str = typer.Option(
        None,
        help="prefix to save"
    ),
):
    """Push a checkpoint to a target"""
    park_url = os.environ.get("PARK_URL")
    api = Park(park_url)

    namespace = target.split("/")[0]
    env_tag = target.split("/")[1]
    environment = env_tag.split(":")[0]
    tag = env_tag.split(":")[1]

    checkpoint_data = api.pull(namespace, environment, tag)

    if prefix is None:
        prefix = os.environ.get("CONDA_PREFIX")
    else:
        prefix = os.path.abspath(prefix)

    chck = Checkpoint.from_checkpoint_dict(checkpoint_data=checkpoint_data, prefix=prefix)
    chck.save()
