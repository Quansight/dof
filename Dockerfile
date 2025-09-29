# This Dockerfile builds the dof-builder image.
# This image provides dof for use in the build stage of
# creating a docker image from a dof checkpoint revision.
#
# To build and share this image run:
#  $ docker build . -t scastellarin/dof-builder:v0.1.0
#  $ docker push scastellarin/dof-builder:v0.1.0
         
FROM continuumio/miniconda3

# Set the working directory in the container
WORKDIR /app

# Copy the application code into the container
COPY . .

# Install
RUN pip install .
