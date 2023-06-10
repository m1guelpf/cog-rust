#!/bin/bash

version=$(git describe --tags | sed 's/^v//;s/\([^-]*-g\)/r\1/')

if [[ $version != *.*.* ]]; then
  version="$version.0"
fi

echo "Bumping Cargo version to $version"

# Replace the version in the Cargo.toml file with the $version variable
sed -i "0,/version = \".*\"/s//version = \"$version\"/" Cargo.toml
