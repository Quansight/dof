class PixiCondaMeta:
    @classmethod
    def detect(cls, prefix):
        """Detect if the given prefix is a pixi based conda meta.
        If it is, it will return an instance of PixiCondaMeta
        """
        # TODO: detect conda-meta
        return None

    def __init__(self, prefix):
        self.prefix = prefix