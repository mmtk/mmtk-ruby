#!/bin/bash

set -xe

# Install early script dependencies.
# These packages are needed before all repositories are checked out.
# For example, python3-tomlkit is used for determining the revision of the ruby repository.
sudo apt-get update -y
sudo apt-get install python3-tomlkit
