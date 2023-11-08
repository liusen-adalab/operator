#!/bin/bash

set -x
server="factory"
ssh $server 'journalctl --output cat  -f -u av1-operator.service'