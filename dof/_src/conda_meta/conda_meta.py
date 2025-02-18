# NOTE: 
# There is a case for refactoring this into a pluggable or hook
# based setup. For the purpose of exploring this approach we 
# won't set that up here.

import os

from dof._src.conda_meta.conda import CondaCondaMeta
from dof._src.conda_meta.pixi import PixiCondaMeta


class CondaMeta():
    def __init__(self, prefix):
        """CondaMeta provides a way of interacting with the
        conda-meta directory of an environment. Tools like conda
        and pixi use conda-meta to keep important metadata about
        the environment and it's history.

        Parameters
        ----------
        prefix: str
            The path to the environment
        """
        self.prefix = prefix

        if not os.path.exists(prefix):
            raise Exception(f"prefix {prefix} does not exist")

        if not os.path.exists(f"{prefix}/conda-meta"):
            raise Exception(f"invalid environment at {prefix}, conda-meta dir does not exist")

        # detect which conda-meta flavour is used by the environment
        for impl in [CondaCondaMeta, PixiCondaMeta]:
            self.conda_meta = impl.detect(prefix)
            if self.conda_meta is not None:
                break

        # if none is detected raise an exception
        if self.conda_meta is None:
            raise Exception("Could not detect conda or pixi based conda meta")

    def get_requested_specs(self) -> list[str]:
        """Return a list of all the specs a user requested to be installed.

        A user_requested_spec is one that the user explicitly asked to be 
        installed. These are different from dependency_specs which are specs
        that are installed because they are dependencies of the
        requested_specs.

        For example, when a user runs `conda install flask`, the user requested
        spec is flask. And all the other installed packages are dependency_specs

        Returns
        -------
        specs: list[str]
            A list of all the specs a user requested to be installed.
        """
        return self.conda_meta.get_requested_specs()
