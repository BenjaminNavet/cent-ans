#!/usr/bin/env bash
# Captures de l'interface de campagne aux quatre résolutions de référence (audit A3, lot U4).
# Les invariants (échelle, bouton ×, minicarte, exclusivité) sont vérifiés en headless par le
# smoke (`_run_ui_layout`, seul : CENT_ANS_SMOKE_ONLY=ui_layout) ; ce script produit les images
# à regarder, une fenêtre réelle étant nécessaire au rendu.
#
# Usage : game/tests/ui_resolutions.sh [dossier] [étapes…]
#   dossier : défaut docs/audit/captures/ui2 ; étapes : défaut « map budget court tech ».
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="${1:-$root/docs/audit/captures/ui2}"
shift || true
stages=("$@")
if [ ${#stages[@]} -eq 0 ]; then
	stages=(map budget court tech)
fi
mkdir -p "$out"
out="$(cd "$out" && pwd)"
status=0
for resolution in 1280x720 1440x900 1920x1080 2560x1440; do
	for stage in "${stages[@]}"; do
		shot="$out/u4-$stage-$resolution.png"
		if godot --resolution "$resolution" --path "$root/game" res://scenes/campaign_map.tscn -- \
			--stage="$stage" --screenshot="$shot" > /dev/null 2>&1 && [ -f "$shot" ]; then
			echo "ok  $shot"
		else
			echo "ÉCHEC $stage à $resolution" >&2
			status=1
		fi
	done
done
exit $status
