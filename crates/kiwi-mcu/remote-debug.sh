#!/bin/bash
set -e

PI_HOST="clement@192.168.1.133"
REMOTE_DIR="/tmp/kiwi-mcu"
CHIP="STM32F767ZITx"

BIN_NAME=$(basename "$1")
REMOTE_ELF="$REMOTE_DIR/$BIN_NAME"

ssh "$PI_HOST" "source ~/.cargo/env && killall -q probe-rs 2>/dev/null || true; mkdir -p $REMOTE_DIR"
rsync -az --inplace "$1" "$PI_HOST:$REMOTE_ELF"

cleanup() {
    ssh "$PI_HOST" "source ~/.cargo/env && killall -q probe-rs 2>/dev/null || true"
}
trap cleanup EXIT INT TERM

ssh "$PI_HOST" "source ~/.cargo/env && probe-rs download --chip $CHIP $REMOTE_ELF"

ssh -t "$PI_HOST" "source ~/.cargo/env && probe-rs gdb --chip $CHIP --gdb-connection-string '0.0.0.0:3333'"