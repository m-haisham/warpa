#!/bin/bash

cargo build -p warpa --release

echo "rust:warpa"
time (./target/release/warpa extract rpa/archive.rpa -o rpa/output/warpa)

echo ""
echo ""
echo "python:rpatool"
time (python rpatool.py -x rpa/archive.rpa -o rpa/output/rpatool)

