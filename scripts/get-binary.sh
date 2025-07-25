#!/bin/sh

# Fetches the release binary for the tide tracker project
curl -L https://github.com/lcary/tide-tracker-rs/releases/download/v0.3/tide-tracker > tide-tracker
chmod +x ./tide-tracker
