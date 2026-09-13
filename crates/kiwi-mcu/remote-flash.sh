#!/bin/bash
set -e

PI_HOST="clement@kiwi.local"
REMOTE_DIR="/tmp/kiwi-mcu"
CHIP="STM32F767ZITx"

LOCAL_ELF="$1"
BIN_NAME=$(basename "$LOCAL_ELF")

ssh "$PI_HOST" "mkdir -p $REMOTE_DIR"
rsync -az --inplace "$LOCAL_ELF" "$PI_HOST:$REMOTE_DIR/$BIN_NAME"

ssh -t "$PI_HOST" "source ~/.cargo/env && probe-rs download --chip $CHIP $REMOTE_DIR/$BIN_NAME"