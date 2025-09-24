from enum import Enum


DEFAULT_DOCKER_EXPORT_BASE_IMAGE = "ubuntu:24.04"

DOCKER_EXPORT_TEMPLATE = """
FROM scastellarin/dof-builder:v0.1.0 AS build

WORKDIR /tmp

COPY {TARGET_CHECKPOINT_FILE} ./checkpoint

RUN dof install-checkpoint --file ./checkpoint --prefix /tmp/env

FROM {BASE_IMAGE} AS prod

COPY --from=build /tmp/env /usr/local/env

ENV PATH=/usr/local/env/bin:${PATH}
"""

class SupportedExportFormats(str, Enum):
    DOCKER = "docker"
