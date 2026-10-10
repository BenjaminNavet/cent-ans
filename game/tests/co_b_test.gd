extends TestCase

## Test headless du lot CO-B (pastilles des colonies de la province à la sélection) :
##  1. `SettlementData.province_mates` : colonies de la province d'une colonie donnée ;
##  2. les hameaux (`hamlets`) ne sont ni des colonies ni sélectionnables (hors `index_by_id`,
##     donc hors du pick qui ne parcourt que `settlements`) ;
##  3. `ProvinceMatesOverlay` : libellé du cartouche, pastilles créées puis masquées.
## Usage : godot --headless --path game --script res://tests/co_b_test.gd


func _init() -> void:
	await process_frame
	_run()
	finish()


func _run() -> void:
	var data_dir := MAP_PATHS.default_data_dir()
	var data := SettlementData.load_from(data_dir, data_dir.path_join("map"))
	var paris := data.get_settlement("set_paris")
	if not check(not paris.is_empty(), "set_paris missing"):
		return
	var mates := data.province_mates("set_paris")
	check(mates.size() >= 1, "Paris province has colonies")
	var seen_self := false
	for entry in mates:
		check(str(entry["province"]) == str(paris["province"]), "mate in same province")
		seen_self = seen_self or str(entry["id"]) == "set_paris"
	check(seen_self, "selected settlement is listed")
	check(data.province_mates("set_inconnu").is_empty(), "unknown id gives empty list")
	# Hameaux non sélectionnables.
	check(data.hamlets.size() > 0, "hamlets loaded")
	var settlement_names := {}
	for entry in data.settlements:
		settlement_names[str(entry["id"])] = true
	var leaked := 0
	for hamlet in data.hamlets:
		if data.index_by_id.has(str(hamlet.get("id", ""))) or hamlet.has("kind"):
			leaked += 1
	check(leaked == 0, "hamlets are never in the pickable settlement index")
	check(data.index_by_id.size() == data.settlements.size(), "index covers settlements only")
	# Cartouche.
	check(ProvinceMatesOverlay.cartouche_text("Île-de-France", 5) == "Province d’Île-de-France — 5 colonies", "cartouche plural")
	check(ProvinceMatesOverlay.cartouche_text("X", 1) == "Province de X — 1 colonie", "cartouche singular")
	# Pastilles (sans carte : couche factice).
	var layer := SettlementLayer.new()
	layer.data = data
	var overlay := ProvinceMatesOverlay.new()
	root.add_child(overlay)
	overlay.setup(layer, null, overlay, func(p: String) -> String: return p)
	if mates.size() >= 2:
		overlay.show_for("set_paris")
		check(overlay.mates.size() == mates.size(), "overlay lists the province colonies")
		layer.selection_cleared.emit()
		check(overlay.mates.is_empty(), "overlay hidden on deselection")
