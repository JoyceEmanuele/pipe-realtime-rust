#!/bin/bash
RUSTFLAGS=-Awarnings cargo build --release || exit 1

CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-./target}

echo "Testando as configs de exemplo..."
$CARGO_TARGET_DIR/release/realtime --test-config --config-example

echo ""
echo "Testando as configs atuais..."
$CARGO_TARGET_DIR/release/realtime --test-config

cp -f $CARGO_TARGET_DIR/release/realtime .
