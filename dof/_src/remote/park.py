import requests


class Park:
    """API for interacting with Park backend"""

    def __init__(self, url: str):
        self.url = url

    