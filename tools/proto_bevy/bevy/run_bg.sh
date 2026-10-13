#!/bin/zsh
# Builds proto-bevy (release), wraps it in a minimal .app bundle and launches it with
# `open -g -W` so no window ever takes the player's focus (mirror of tools/godot_bg.sh).
# The log (stdout + stderr of the app) is printed on the standard output when the app exits.
# Usage: tools/proto_bevy/bevy/run_bg.sh [--metrics f.csv] [--shot f.png --at 5] [--dump-state f.json] [--set k=v ...]
# Relative output paths are resolved from the current directory (the app starts in /).
# Env: PROTO_BEVY_FEATURES=dev-fast builds with Bevy dynamic linking (compile-time criterion).
# Exit code: 1 if the app logged PROTO_BEVY_FATAL (e.g. instanced batches not drawn), else 0.
set -u
here=${0:A:h}
(
	cd "$here" || exit 1
	if [[ -n "${PROTO_BEVY_FEATURES:-}" ]]; then
		cargo build --release --features "$PROTO_BEVY_FEATURES"
	else
		cargo build --release
	fi
) >&2 || exit 1

app="$here/target/ProtoBevy.app"
mkdir -p "$app/Contents/MacOS"
cp "$here/target/release/proto-bevy" "$app/Contents/MacOS/proto-bevy"
cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key><string>proto-bevy</string>
	<key>CFBundleIdentifier</key><string>fr.centans.proto-bevy</string>
	<key>CFBundleName</key><string>ProtoBevy</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleVersion</key><string>1</string>
	<key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign - "$app" >/dev/null 2>&1 || true

args=()
while (( $# )); do
	case "$1" in
	--metrics | --shot | --dump-state | --scene)
		args+=("$1" "${2:A}")
		shift 2
		;;
	*)
		args+=("$1")
		shift
		;;
	esac
done

log=$(mktemp -t proto_bevy)
open -g -n -W --stdout "$log" --stderr "$log" "$app" --args "${args[@]}"
cat "$log"
fatal=0
grep -q -e PROTO_BEVY_FATAL -e "Quitting the application due to" "$log" && fatal=1
rm -f "$log"
# `open` does not transmit the exit code: a fatal marker in the log becomes exit 1.
(( fatal )) && exit 1
exit 0
