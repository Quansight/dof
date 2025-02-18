import os
from conda.core import envs_manager
from conda.history import History

class CondaCondaMeta:
    @classmethod
    def detect(cls, prefix):
        """Detect if the given prefix is a conda based conda meta.
        If it is, it will return an instance of CondaCondaMeta
        """
        known_prefixes = envs_manager.list_all_known_prefixes()
        if prefix in known_prefixes:
            return cls(prefix)
        return None

    def __init__(self, prefix):
        self.prefix = prefix
        history_file = f"{prefix}/conda-meta/history"
        if not os.path.exists(history_file):
            raise Exception(f"history file for prefix '{prefix}' does not exist")
        self.history = History(prefix)

    def get_requested_specs(self) -> list[str]:
        """Return a list of all the specs a user requested to be installed.
        Returns
        -------
        specs: list[str]
            A list of all the specs a user requested to be installed.
        """
        requested_specs = self.history.get_requested_specs_map()
        return [spec.dist_str() for spec in requested_specs.values()]