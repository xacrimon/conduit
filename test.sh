#!/bin/bash

set -ex

cargo build
size=$(ls -la target/debug | grep conduit | grep '\-rwxr\-xr\-x' | awk '{print $5}')

if [ "$size" -gt "20000000" ]; then
   exit 1
fi

exit 0
