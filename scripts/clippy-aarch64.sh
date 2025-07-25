#!/bin/sh
cross clippy --target=aarch64-unknown-linux-gnu --all-features -- -D warnings
