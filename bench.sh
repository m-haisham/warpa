#!/bin/bash

cargo build --release

echo "rust:rpalib"
time (./target/release/rpalib)

echo ""
echo ""
echo "python:rpatool"
time (python rpatool.py -x test.rpa)