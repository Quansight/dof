from typing import Optional

import typer

app = typer.Typer()


@app.command()
def hello(name: Optional[str] = None):
    """Demo: say hello
    """
    if name:
        typer.echo(f"Hello {name}")
    else:
        typer.echo("Hello World!")


@app.command()
def install(
    path: str = typer.Option(
        None,
        help="path to lockfile"
    ),
):
    """Install a lockfile
    """
    if path is None:
        raise Exception("path is required to install")
    
    print("not really installing")


@app.command()
def lock(
    output: str = typer.Option(
        None,
        help="path to output lockfile"
    ),
):
    """Generate a lockfile
    """
    if output is None:
        raise Exception("path is required to install")
    
    print("not really locking")