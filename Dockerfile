FROM continuumio/miniconda3

# Set the working directory in the container
WORKDIR /app

# Copy the application code into the container
COPY . .

# Install
RUN pip install .