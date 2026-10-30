#!/bin/zsh
# Liste les vérifications utiles pour les fichiers modifiés entre <base> et HEAD (agent de fusion).
# Usage : tools/checks_for_diff.sh <base>      ex. tools/checks_for_diff.sh main
# Sortie : une ligne `RUN  <nom>  <commande>  # <fichiers déclencheurs>` par vérification utile, puis
#          `SKIP <noms>` pour les autres. Ne lance rien ; les commandes lourdes passent par tools/heavy.sh.
set -u
[[ $# -eq 1 ]] || { echo "usage: tools/checks_for_diff.sh <base>" >&2; exit 2; }
root=${0:A:h:h}; cd $root || exit 1
files=("${(@f)$(git diff --name-only $1...HEAD)}")
files=(${files:#})
(( $#files )) || { echo "no changes between $1 and HEAD"; exit 0; }

# Chemins des économie/politique de campagne (données de règles hors bataille et siège).
campaign_rules='^data/rules/(acts|advisor|agenda|agents|armies|army_traditions|assemblies|campaign_weather|capture|cause_thresholds|crusade|diplomacy|dynasty|economy|encounters|feudal|finance|loyalty|mercenaries|population|religion|replenishment|revolt|starting_armies|starting_fit|trait_triggers|difficulty)\.json$'
campaign_data='^data/(economy|buildings|provinces|factions|settlements|resources|diets|titles|characters|technologies|edicts|royal_acts|events|council|balance|chivalric_orders|religions|traits|skills|movement)/'
battle_files='^(core/crates/(sim-battle|ai)/|data/rules/battle_|data/unit_types/|data/battle_)'
fixture_files='^(data/unit_types/|data/rules/battle_)'
campaign_files="^core/crates/(sim-campaign|ai|game-api)/|$campaign_rules|$campaign_data"

declare -A why   # nom -> fichiers déclencheurs (3 premiers)
declare -a order
hit() {  # hit <nom> <regex> : retient la vérification si un fichier modifié correspond
	local selected=() f
	for f in $files; do [[ $f =~ $2 ]] && selected+=$f; done
	(( $#selected )) || return 1
	why[$1]="${(j:, :)selected[1,3]}"
	(( $#selected > 3 )) && why[$1]+=" (+$(( $#selected - 3 )))"
	order+=$1
}

non_docs=(${files:#docs/*})
(( $#non_docs )) && { why[full_check]="${(j:, :)non_docs[1,3]}"; order+=full_check; }
hit battle_bench $battle_files
hit auto_resolve_fixture $fixture_files
hit eco_series $campaign_files
hit seed_bots_quick '^(core/|data/)'
hit smoke '^(client/|game/)'
hit pytest '^tools/'

typeset -A cmd=(
	full_check       "tools/full_check.sh out/check"
	battle_bench     "tools/heavy.sh cargo run --release -p sim-battle --example battle_bench -- --smoke   (en entier si les règles de bataille changent)"
	auto_resolve_fixture "tools/heavy.sh cargo run --release -p sim-battle --example battle_bench -- --emit-auto-fixture   (puis juger l'accord, ADR 0807)"
	eco_series       "tools/heavy.sh cargo run --release -p ai --example eco_series -- --seeds 1,2,3"
	seed_bots_quick  "(cd core && ../tools/heavy.sh cargo run --profile seed-bots -p seed-bots -- run --quick)"
	smoke            "CENT_ANS_MUTE=1 client/tests/smoke.sh"
	pytest           "uv run --project tools pytest"
)
echo "base=$1 files=$#files"
for name in $order; do printf 'RUN  %-22s %s  # %s\n' $name $cmd[$name] $why[$name]; done
skipped=()
for name in full_check battle_bench auto_resolve_fixture eco_series seed_bots_quick smoke pytest; do
	(( ${order[(Ie)$name]} )) || skipped+=$name
done
(( $#skipped )) && echo "SKIP ${skipped}"
exit 0
