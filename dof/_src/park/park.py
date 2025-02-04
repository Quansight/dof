import requests
import json


class Park:
    """API for interacting with Park backend"""

    def __init__(self, url: str):
        self.url = url

    def push(self, namespace: str, environment: str, checkpoint: str, data: dict):
        request_url = f"{self.url}/{namespace}/{environment}/{checkpoint}/json"
        response = requests.post(
            request_url,
            data=json.dumps(data),
            headers={"Content-Type": "application/json"}
        )
        response.raise_for_status()
        return response.json()

    def pull(self, namespace: str, environment: str, checkpoint: str):
        request_url = f"{self.url}/{namespace}/{environment}/{checkpoint}"
        response = requests.get(request_url)
        response.raise_for_status()
        data = response.json()
        return data["data"]["checkpoint_data"]
