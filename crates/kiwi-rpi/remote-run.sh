#!/bin/bash
set -e

PI_HOST="clement@kiwi.local"
REMOTE_DIR="\$HOME/kiwi-rpi"

LOCAL_BIN="$1"
BIN_NAME=$(basename "$LOCAL_BIN")

ssh "$PI_HOST" "mkdir -p $REMOTE_DIR"
rsync -az --inplace "$LOCAL_BIN" "$PI_HOST:$REMOTE_DIR/$BIN_NAME"

ssh -t "$PI_HOST" "cd $REMOTE_DIR && ./$BIN_NAME"