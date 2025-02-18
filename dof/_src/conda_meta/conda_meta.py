# NOTE: 
# There is a case for refactoring this into a pluggable or hook
# based setup. For the purpose of exploring this approach we 
# won't set that up here.

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
        return []
