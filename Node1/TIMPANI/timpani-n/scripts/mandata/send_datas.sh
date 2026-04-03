#!/bin/bash

# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT

workdir=$PWD
c_date=`date +%y%m%d%H%M`

f_gpdata=node$1.$c_date.gpdata
f_trace=trace.node$1.$c_date
sendto=$2

sudo cp /sys/kernel/debug/tracing/trace $workdir/$f_trace
#sudo sh -c "echo > /sys/kernel/debug/tracing/trace"
sudo mv $workdir/build/node$1.gpdata $workdir/$f_gpdata

sudo chmod 644 $workdir/$f_gpdata $workdir/$f_trace
scp $workdir/$f_gpdata $workdir/$f_trace $sendto
sudo rm $workdir/$f_gpdata $workdir/$f_trace
