import hashlib


def hash_string(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def get_name_from_prefix(prefix: str) -> str:
    """This function assumes an environment name is the last word in a conda prefix"""
    return prefix.split("/")[-1]
