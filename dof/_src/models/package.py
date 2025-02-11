from pydantic import BaseModel


class CondaPackage(BaseModel):
    name: str
    version: str
    build: str
    platform: str
    conda_channel: str
    hash: dict[str, str]


class PipPackage(BaseModel):
    name: str
    version: str
    build: str
    url: str | None = None

    def __str__(self) -> str:
        return f"pip: {self.name} - {self.version}"


class UrlPackage(BaseModel):
    url: str

    def __str__(self) -> str:
        package = self.url.split("/")[-1]
        version = package.split("-")[-2]
        name = "-".join(package.split("-")[:-2])
        return f"conda: {name} - {version}"


Package = CondaPackage | PipPackage | UrlPackage
