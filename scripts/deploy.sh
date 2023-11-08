#!/bin/bash
# Description: Deploy av1-operator
#

echo "Deploying av1-operator manager..."

set -e
set -x


# config
bin_name=av1-operator
host="root@10.0.20.1"
service_name=av1-operator

# build
cargo build --release

# init remote environment
etc_path=/etc/${service_name}
ssh ${host} "mkdir -p ${etc_path}/configs"

# sync app configs
scp -rO configs/* ${host}:/etc/${service_name}/configs >/dev/null 2>&1
# sync systemd config
scp -O ./configs/${service_name}.service ${host}:/etc/systemd/system/${service_name}.service >/dev/null 2>&1

# stop service
ssh ${host} "systemctl stop ${service_name} || true"
# sync binary
scp -O ./target/release/${bin_name} ${host}:/usr/local/bin/${service_name} >/dev/null 2>&1

# start service
ssh ${host} "systemctl daemon-reload"
ssh ${host} "systemctl enable ${service_name} --now"

# check service status
ssh ${host} "systemctl status ${service_name}"
