#!/usr/bin/env bash
# Double-click launcher (ADR 0117): rebuilds and reimports what changed, then starts the game.
cd "$(dirname "$0")" && exec tools/launch.sh "$@"
