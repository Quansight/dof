class CondaCondaMeta:
    @classmethod
    def detect(cls, prefix):
        """Detect if the given prefix is a conda based conda meta.
        If it is, it will return an instance of CondaCondaMeta
        """
        # TODO: detect conda-meta
        return cls(prefix)

    def __init__(self, prefix):
        self.prefix = prefix