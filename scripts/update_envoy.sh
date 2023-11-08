#!/bin/bash
set -e
set -x

host="root@10.0.20.1"

#############################

# build
cargo build -p av1-envoy --release 
# sync
built_out_path=./target/release/av1-envoy
target_path="/var/lib/av1-operator/bin/av1-envoy"
scp -O $built_out_path ${host}:${target_path} >/dev/null 2>&1
