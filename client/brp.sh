#!/bin/zsh
# JSON-RPC call to a running dev client (feature `dev`, BRP on localhost). Zero tool-schema tokens,
# unlike an MCP server. Launch the client first with client/run_bg.sh (offscreen, muted):
#   CENT_ANS_MUTE=1 CENT_ANS_FEATURES=dev CENT_ANS_PROFILE=debug CENT_ANS_BRP_PORT=15742 \
#     client/run_bg.sh --offscreen --state Campaign --exit-after 100000 > out/run.log 2>&1 &
# Usage: client/brp.sh <method> [params-json]       (port: $CENT_ANS_BRP_PORT, default 15702)
#   brp.sh cent_ans/dump                            same JSON as --dump-state, live
#   brp.sh brp_extras/send_keys '{"keys":["F4"]}'   key press (also type_text, click_mouse...)
#   brp.sh brp_extras/screenshot '{"path":"/abs/out/a.png","camera":<entity>}'
#       (the primary window is black offscreen: pass the entity of a camera, see world.query)
#   brp.sh brp_extras/shutdown                      clean exit
#   brp.sh rpc.discover                             every method
set -u
port=${CENT_ANS_BRP_PORT:-15702}
method=$1
params=${2:-}
if [[ -n $params ]]; then
	body="{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$method\",\"params\":$params}"
else
	body="{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$method\"}"
fi
curl -sS -m 60 "localhost:$port" -d "$body"
echo
