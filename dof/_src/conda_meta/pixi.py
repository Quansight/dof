import os

class PixiCondaMeta:
    @classmethod
    def detect(cls, prefix):
        """Detect if the given prefix is a pixi based conda meta.
        If it is, it will return an instance of PixiCondaMeta
        """
        conda_meta_path = f"{prefix}/conda-meta"
        # if the {prefix}/conda-meta/pixi path exists, then this is
        # a pixi based conda meta environment
        if os.path.exists(f"{conda_meta_path}/pixi"):
            return cls(prefix)
        return None

    def __init__(self, prefix):
        self.prefix = prefix

    # TODO
    def get_requested_specs(self) -> list[str]:
        """Return a list of all the specs a user requested to be installed.
        Returns
        -------
        specs: list[str]
            A list of all the specs a user requested to be installed.
        """
        return []