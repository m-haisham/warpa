#!/bin/bash

cargo build -p warpa --release

echo "rust:warpa"
time (./target/release/warpa x test.rpa)

echo ""
echo ""
echo "python:rpatool"
time (python rpatool.py -x test.rpa)