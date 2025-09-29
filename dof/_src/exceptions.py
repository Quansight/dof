class DockerBuildFailed(Exception):
    def __init__(self, command, cwd, err):
        self.msg = (
            f"Failed to build docker image!"
            f"\nRan command: `{' '.join(command)}`"
            f"\ncwd: `{cwd}`"
            f"\nError message: {err}"
        )
        super().__init__(self.msg)
