#!/bin/zsh
# Builds the Cent Ans Bevy client, wraps it in a minimal .app bundle ("CentAnsBevy") and launches
# it with `open -g -n -W` so no window ever takes the player's focus (mirror of tools/godot_bg.sh).
# The app log (stdout + stderr) is printed on the standard output when the app exits.
# Usage: client/run_bg.sh [--state S] [--faction F] [--seed N] [--dump-state f.json] [--shot f.png]
#                         [--frames N] [--exit-after N] [--script f.txt] [--offscreen]
# Relative paths (flags, and `dump`/`shot` lines of the script) are resolved from the current
# directory, since the app starts in /.
# Env: CENT_ANS_FEATURES=dev       cargo features (e.g. dev = Bevy dynamic linking)
#      CENT_ANS_PROFILE=debug      cargo profile dir (default: release)
#      Every CENT_ANS_* variable is forwarded to the app (`open --env`).
# Exit code: 1 if the log has a panic / AGENT_FATAL marker, else 0 (`open` hides the app's code).
set -u
here=${0:A:h}
profile=${CENT_ANS_PROFILE:-release}
build=(cargo build)
[[ $profile == release ]] && build+=(--release)
[[ -n "${CENT_ANS_FEATURES:-}" ]] && build+=(--features "$CENT_ANS_FEATURES")
(cd "$here" && "${build[@]}") >&2 || exit 1

app="$here/target/CentAnsBevy.app"
mkdir -p "$app/Contents/MacOS"
cp "$here/target/$profile/cent-ans" "$app/Contents/MacOS/cent-ans"
cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key><string>cent-ans</string>
	<key>CFBundleIdentifier</key><string>fr.centans.bevy</string>
	<key>CFBundleName</key><string>CentAnsBevy</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleVersion</key><string>1</string>
	<key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign - "$app" >/dev/null 2>&1 || true

args=()
tmp_script=""
while (( $# )); do
	case "$1" in
	--dump-state | --shot)
		args+=("$1" "${2:A}")
		shift 2
		;;
	--script)
		# Copy the script with absolute dump/shot paths (the app's working directory is /).
		tmp_script=$(mktemp -t cent_ans_script)
		awk -v cwd="$PWD" '
			($1 == "dump" || $1 == "shot") && $2 !~ /^\// { print $1, cwd "/" $2; next }
			{ print }' "$2" > "$tmp_script"
		args+=("$1" "$tmp_script")
		shift 2
		;;
	*)
		args+=("$1")
		shift
		;;
	esac
done

# `open` does not forward the shell environment: pass every CENT_ANS_* variable explicitly.
env_args=()
for name in ${(k)parameters[(I)CENT_ANS_*]}; do
	env_args+=(--env "$name=${(P)name}")
done

log=$(mktemp -t cent_ans_log)
open -g -n -W "${env_args[@]}" --stdout "$log" --stderr "$log" "$app" --args "${args[@]}"
cat "$log"
fatal=0
grep -q -e AGENT_FATAL -e "panicked at" -e "Quitting the application due to" "$log" && fatal=1
rm -f "$log" ${tmp_script:+"$tmp_script"}
(( fatal )) && exit 1
exit 0
