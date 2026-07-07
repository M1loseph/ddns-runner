#!/bin/bash
set -e 

GIT_VERSION=$(git describe --tags --abbrev=0)

if [[ ! "$GIT_VERSION" =~ ^ddns-runner/.* ]]
then
    echo "Incorrect version $GIT_VERSION"
    exit 1
fi

PURE_VERSION=$(echo "$GIT_VERSION" | cut -d '/' -f 2)

echo "$PURE_VERSION"
