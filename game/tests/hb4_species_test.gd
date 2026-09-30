extends SceneTree

## Test headless du lot HB4 (essences et répartition par biome, ADR 0143) :
##  1. catalogue `data/art/tree_species.json` chargé (≥ 16 essences, lignes 0-2 chêne/hêtre/sapin) ;
##  2. atlas GA3 complet : une ligne de 8 azimuts par essence, chaque cellule couverte ;
##  3. semis d'une tuile témoin par biome (biomes.png synthétique) : ≥ 2 essences par biome ;
##  4. densité hors forêt (champs) sous un seuil, steppe presque nue ;
##  5. nombre d'instances ≤ +30 % par rapport au semis sans essences (table vide).
## Usage : godot --headless --path game --script res://tests/hb4_species_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	print("hb4_species_test: skeleton (disabled)")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition
