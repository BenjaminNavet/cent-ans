extends TestCase

## Lot RX anim (ADR 0250-0251) : seuils d'allure et cadence nominale cohérents, jitter réduit en
## locomotion, cadence par unité, mêlée des archers / de la cavalerie, cycle = durée des clips,
## réactions aux pertes, clips de rôle `melee` / `routing`.
## Usage : godot --headless --path game --script res://tests/rx_anim_test.gd


func _init() -> void:
	var gore := BattleGore.settings()
	var cadence: Dictionary = gore.get("cadence", {})
	var gaits: Dictionary = BattleSkinned.animation_settings().get("cavalry_gaits", {})
	var trot_min := float(gaits["trot_min_speed"])
	var gallop_min := float(gaits["gallop_min_speed"])
	var hyst := float(gaits.get("hysteresis_mps", 0.4))
	# Facteur de lecture à l'entrée dans chaque allure : jamais « accéléré » à l'entrée du galop
	# (≤ 1,1), trot entre 0,75 et 1,35 sur toute sa bande, pas jamais au-delà du plafond 1,8.
	var gallop_entry := (gallop_min + hyst) / float(cadence["c_gallop"])
	check(gallop_entry >= 0.9 and gallop_entry <= 1.1, "galop à l’entrée lu à x%.2f" % gallop_entry)
	check(float(cadence["c_charge"]) == float(cadence["c_gallop"]), "charge et galop : même cadence nominale")
	var trot_low := (trot_min + hyst) / float(cadence["c_trot"])
	var trot_high := (gallop_min + hyst) / float(cadence["c_trot"])
	check(trot_low >= 0.75 and trot_high <= 1.4, "trot x%.2f..x%.2f" % [trot_low, trot_high])
	var walk_high := (trot_min + hyst) / float(cadence["c_walk"])
	check(walk_high <= 1.8, "pas au seuil de trot x%.2f > plafond 1,8" % walk_high)
	check(gallop_min > trot_min, "seuils ordonnés")
	# Jitter : réduit en locomotion, plein pour les attentes.
	var walk := BattleSkinned.state_config("infantry", 0, "marching", false)
	var idle := BattleSkinned.state_config("infantry", 0, "idle", false)
	check(absf(float(walk["jitter"]) - 0.03) < 0.001, "jitter de marche %s" % walk["jitter"])
	check(absf(float(idle["jitter"]) - 0.07) < 0.001, "jitter d’attente %s" % idle["jitter"])
	var trot := BattleSkinned.state_config("cavalry", 0, "trotting", false)
	check(absf(float(trot["jitter"]) - 0.03) < 0.001, "jitter de trot")
	# Cadence par unité : facteur absent = 1.
	check(is_equal_approx(BattleSoldiers.nominal_cadence(gore, "walk", "unit_inconnue"), float(cadence["walk"])), "unité inconnue = table par clip")
	var heavy := BattleSoldiers.nominal_cadence(gore, "walk", "unit_men_at_arms_foot")
	check(heavy < float(cadence["walk"]), "fantassin lourd : foulée plus courte")
	# Mêlée : archers sans clips d'épée, cavalerie variée, cycle >= clip le plus long, hit.
	for variant in [0, 1]:
		var kind := "archer%d" % variant
		var config := BattleSkinned.state_config("archer", variant, "melee", false)
		if config["names"].is_empty():
			continue
		check(not (config["names"] as Array).has("slash"), "%s en mêlée sans slash: %s" % [kind, config["names"]])
		check(not (config["names"] as Array).has("hit"), "%s : hit hors du tirage de base" % kind)
		check((config["hit"] as Array).size() > 0, "%s : jeu de réactions aux pertes" % kind)
	var rig_entry := BattleSkinned.rig("cavalry", 0)
	for variant in 7:
		var config := BattleSkinned.state_config("cavalry", variant, "melee", false)
		var distinct := {}
		for c in config["names"]:
			distinct[str(c)] = true
		check(distinct.size() >= 4, "cavalry_%d mêlée: %d clips distincts" % [variant, distinct.size()])
		var clips: Dictionary = BattleSkinned.rig("cavalry", variant).get("clips", {})
		for c in config["names"]:
			var info: Dictionary = clips.get(str(c), {})
			if not bool(info.get("loop", false)):
				check(float(config["cycle"]) + 0.001 >= float(info.get("frames", 0)) / 24.0, "cavalry_%d cycle %.2f < %s" % [variant, config["cycle"], c])
	var sword := BattleSkinned.state_config("infantry", 0, "melee", false)
	var sword_clips: Dictionary = BattleSkinned.rig("infantry", 0).get("clips", {})
	for c in sword["names"]:
		var info: Dictionary = sword_clips.get(str(c), {})
		if not bool(info.get("loop", false)):
			check(float(sword["cycle"]) + 0.001 >= float(info.get("frames", 0)) / 24.0, "cycle %.2f coupe %s" % [sword["cycle"], c])
	check(rig_entry.has("clips"), "rig cavalerie")
	# Clips de rôle melee / routing déclarés et présents.
	for role in ["standard", "drum", "horn"]:
		var wanted: Dictionary = (BattleSkinned.animation_settings().get("role_clips", {}) as Dictionary).get(role, {})
		check(wanted.has("melee") and wanted.has("routing"), "role_clips %s melee/routing" % role)
		var names: Array = BattleStandards.role_set(role)
		check(BattleStandards.clip_for(role, "melee", false) < names.size(), "%s melee dans le jeu" % role)
		check(BattleStandards.clip_for(role, "routing", false) < names.size(), "%s routing dans le jeu" % role)
	finish()
