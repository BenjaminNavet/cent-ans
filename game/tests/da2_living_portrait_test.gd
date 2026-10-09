extends TestCase

## DA2 : tests headless des portraits vivants (résolution déterministe, vieillissement, marques,
## cadre). Usage : `godot --headless --path game --script res://tests/da2_living_portrait_test.gd`


func _init() -> void:
	await process_frame
	LivingPortrait.sim_override = null
	LivingPortrait.store_override = null
	_test_axes()
	_test_hash_is_stable()
	_test_archetype_resolution()
	_test_historical_aging()
	_test_marks()
	await _test_frame()
	finish()


func _test_axes() -> void:
	check(LivingPortrait.age_band(8) == "child", "8 → child")
	check(LivingPortrait.age_band(15) == "child", "15 → child")
	check(LivingPortrait.age_band(16) == "young", "16 → young")
	check(LivingPortrait.age_band(45) == "adult", "45 → adult")
	check(LivingPortrait.age_band(70) == "old", "70 → old")
	check(LivingPortrait.culture_of("fac_england") == "england", "England culture")
	check(LivingPortrait.culture_of("fac_milan") == "italy_empire", "Milan culture")
	check(LivingPortrait.culture_of("fac_unknown") == "france", "default culture")
	var knight := {"id": "chr_gen_1", "sex": "male", "age": 40, "army": "army_1", "faction": "fac_france"}
	check(LivingPortrait.rank_of(knight) == "knight", "general → knight")
	check(LivingPortrait.rank_of(knight, {"ruler": "chr_gen_1"}) == "sovereign", "ruler → sovereign")
	var queen := {"id": "chr_gen_2", "sex": "female", "age": 30, "spouse": "chr_gen_1"}
	check(LivingPortrait.rank_of(queen, {"ruler": "chr_gen_1"}) == "sovereign", "consort → sovereign")
	check(LivingPortrait.rank_of({"id": "x", "sex": "male", "age": 50, "title": "Évêque de Laon"}) == "prelate", "bishop title")
	check(LivingPortrait.rank_of({"id": "x", "sex": "male", "age": 30, "house": "Valois"}, {"ruler_house": "Valois"}) == "noble", "royal house")
	check(LivingPortrait.rank_of({"id": "x", "sex": "male", "age": 9}) == "child", "child")
	check(LivingPortrait.rank_of({"id": "x", "sex": "male", "age": 40}, {"data_role": "burgher"}) == "burgher", "data role")


func _test_hash_is_stable() -> void:
	# Valeurs de référence FNV-1a 32 bits (indépendantes de la version de Godot).
	check(LivingPortrait.fnv1a("") == 2166136261, "fnv1a empty")
	check(LivingPortrait.fnv1a("a") == 0xE40C292C, "fnv1a 'a'")
	check(LivingPortrait.fnv1a("chr_gen_42") == LivingPortrait.fnv1a("chr_gen_42"), "fnv1a deterministic")


func _test_archetype_resolution() -> void:
	# Sondes validées présentes dans le dépôt : child_male_child_france_0, knight_male_adult_england_0.
	var boy := {"id": "chr_gen_boy", "sex": "male", "age": 7, "faction": "fac_france", "birth_year": 1360}
	var first := LivingPortrait.resolve(boy)
	check(str(first["kind"]) == "archetype", "born-in-game child gets an archetype (%s)" % first)
	check(str(first["path"]).contains("child_male_child_"), "child archetype path %s" % first["path"])
	check(LivingPortrait.resolve(boy) == first, "resolution is deterministic")
	var captain := {"id": "chr_gen_cap", "sex": "male", "age": 38, "faction": "fac_england", "army": "a1"}
	var resolved := LivingPortrait.resolve(captain)
	check(str(resolved["path"]).contains("knight_male_adult_england_"), "English captain → English knight (%s)" % resolved["path"])
	# Même lignée de visage d'une tranche à l'autre : même index quand les cases ont autant de visages.
	var older := captain.duplicate()
	older["age"] = 55
	var aged := LivingPortrait.resolve(older)
	check(str(aged["kind"]) == "archetype" and str(aged["band"]) != "", "aged captain resolved (%s)" % aged)


func _test_historical_aging() -> void:
	# Édouard III, né en 1312 : portrait fixe (jeune) à 25 ans, variante âgée à 62 ans.
	var young := {"id": "chr_edward_iii", "sex": "male", "age": 25, "birth_year": 1312, "faction": "fac_england"}
	var fixed := LivingPortrait.resolve(young, {"data_role": "ruler"})
	check(str(fixed["kind"]) == "fixed", "Edward III at 25 keeps his 1337 portrait (%s)" % fixed)
	var old := young.duplicate()
	old["age"] = 62
	var aged := LivingPortrait.resolve(old, {"data_role": "ruler"})
	check(str(aged["path"]).ends_with("aged/chr_edward_iii_old.jpg"), "Edward III at 62 → aged variant (%s)" % aged["path"])
	# Figure secondaire hors de sa tranche : archétype de son aire (chevalier anglais adulte).
	var minor := {"id": "chr_william_de_bohun", "sex": "male", "age": 40, "birth_year": 1312, "faction": "fac_england"}
	var minor_old := LivingPortrait.resolve(minor, {"data_role": "commander"})
	check(str(minor_old["path"]).contains("knight_male_adult_england_"), "minor figure leaves his fixed portrait when he ages (%s)" % minor_old)
	minor["age"] = 25
	check(str(LivingPortrait.resolve(minor, {"data_role": "commander"})["kind"]) == "fixed", "minor figure in his band keeps his portrait")
	# Sans archétype de son aire (banque incomplète) : il garde son portrait fixe.
	var french := {"id": "chr_godefroy_d_harcourt", "sex": "male", "age": 67, "birth_year": 1300, "faction": "fac_france"}
	var kept := LivingPortrait.resolve(french, {"data_role": "noble"})
	check(str(kept["kind"]) == "fixed" or str(kept["culture"]) == "france", "no foreign dress for a historical figure (%s)" % kept)


func _test_marks() -> void:
	var character := {"id": "x", "sex": "male", "age": 40, "birth_year": 1330, "alive": true, "captive": true,
		"traits": [{"id": "trait_sickly"}, {"id": "trait_wounded"}], "spouse": "y"}
	var lookup := func(id: String) -> Dictionary: return {"id": id, "alive": false, "death_year": 1369}
	var marks := LivingPortrait.marks_for(character, {"rank": "knight", "kind": "archetype"}, lookup)
	check(marks["captive"] and marks["sick"] and marks["wounded"], "captive, sick, wounded marks")
	check(marks["mourning"], "widowed this year → mourning")
	check(not marks["dead"], "alive")
	var dead := LivingPortrait.marks_for({"alive": false}, {})
	check(dead["dead"], "dead mark")
	var crowned := LivingPortrait.marks_for({}, {"rank": "sovereign", "kind": "fixed", "data_role": "heir", "headwear": "none"})
	check(crowned["crown"], "heir turned king gets a crown")
	var painted := LivingPortrait.marks_for({}, {"rank": "sovereign", "kind": "archetype", "headwear": "crown"})
	check(not painted["crown"], "crowned archetype: no extra crown")


func _test_frame() -> void:
	var holder := Control.new()
	holder.size = Vector2(96, 96)
	root.add_child(holder)
	var frame := PortraitFrame.new()
	frame.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	holder.add_child(frame)
	var placed := frame.show_character({"id": "chr_gen_boy", "name": "Louis de Valois", "sex": "male", "age": 7,
		"faction": "fac_france", "alive": false}, "", {"ruler": ""})
	await process_frame
	check(placed and frame.has_portrait(), "frame shows the archetype")
	check(frame.get_node("Picture").material != null, "dead portrait uses the grisaille shader")
	check(not (frame.get_node("Initials") as Label).visible, "initials hidden under a portrait")
	# Anti-clones : jamais de miroir sur un portrait historique ; décision stable par id.
	var boy := {"id": "chr_gen_boy"}
	check(not LivingPortrait.mirrored(boy, {"kind": "fixed"}), "historical portraits are never mirrored")
	check(LivingPortrait.mirrored(boy, {"kind": "archetype"}) == LivingPortrait.mirrored(boy, {"kind": "archetype"}),
		"mirror decision is stable")
	var empty := PortraitFrame.new()
	holder.add_child(empty)
	empty.show_character({"id": "", "name": "Jean Sans Terre", "sex": "female", "age": 40, "faction": "fac_nowhere"}, "", {"ruler": ""})
	# Banque complète (147 images) : toute case a un visage, la culture par défaut couvre les
	# factions inconnues.
	check(empty.has_portrait(), "full bank: default-culture archetype for an unknown faction")
	check(PortraitFrame.initials("Jean de Valois") == "JV", "initials skip particles")
	holder.queue_free()
