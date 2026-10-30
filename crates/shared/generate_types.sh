#!/bin/bash
set -e

export TS_RS_EXPORT_DIR="../../gcs/src/bindings"

rm -rf $TS_RS_EXPORT_DIR

cargo test --features ts