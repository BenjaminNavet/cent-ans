#!/bin/zsh
# Real-window playtest driver (player consent required: takes the focus, muted).
# Runs a client script in a visible window and captures the window region with screencapture
# every few seconds (the in-game `shot` of the swapchain is black on the campaign map).
# Usage: tools/playtest_window.sh <script.txt> <out_prefix> [seconds=60] [interval=5]
set -u
root=${0:A:h:h}
script=${1:?script} prefix=${2:?prefix} secs=${3:-60} every=${4:-5}
mkdir -p "${prefix:h}"
CENT_ANS_MUTE=1 CENT_ANS_NO_BRIEFING=${CENT_ANS_NO_BRIEFING:-1} "$root/client/target/release/cent-ans" \
	--state ${CENT_ANS_START_STATE:-MainMenu} --visible-window --script "$script" > "$prefix.log" 2>&1 &
pid=$!
# Window bounds lookup (compiled once); only the game window region is captured.
helper=${TMPDIR:-/tmp}/cent_ans_window_id
[[ -x $helper ]] || swiftc -O "$root/tools/window_id.swift" -o "$helper" || exit 1
wid=""
for _ in {1..60}; do
	wid=$("$helper" cent-ans)
	[[ -n $wid ]] && break
	sleep 1
done
[[ -z $wid ]] && { echo "no on-screen window" >&2; kill $pid; exit 1; }
osascript -e 'tell application "System Events" to set frontmost of (first process whose name is "cent-ans") to true' >/dev/null 2>&1
n=0
while (( n * every < secs )) && kill -0 $pid 2>/dev/null; do
	osascript -e 'tell application "System Events" to set frontmost of (first process whose name is "cent-ans") to true' >/dev/null 2>&1
	sleep 0.4
	wid=$("$helper" cent-ans)
	[[ -z $wid ]] && break
	screencapture -x -R"${wid#* }" "$prefix-$(printf %02d $n).png"
	sleep $every
	n=$((n + 1))
done
kill $pid 2>/dev/null
grep -E "AGENT_FATAL|panicked" "$prefix.log" && exit 1
exit 0
