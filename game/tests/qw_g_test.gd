extends TestCase

## Lot QW-G (gains rapides de l'audit art, 09/10) :
## - G1 : texte de l'étiquette du terrain (valeurs lues du cœur, signes ▲/▼) ;
## - G2 : mode daltonien en bataille (bleu/vermillon, ennemi reconnaissable sans couleur) ;
## - G3 : règle commune ▲/▼ des infobulles ;
## - G4 : plancher de texte 720p, `Caption` ≥ 15 px dans le thème et dans `UiType`, aucune
##   taille de police du thème parchemin sous ce plancher.
## Usage : godot --headless --path game --script res://tests/qw_g_test.gd

const MIN_FONT := 15
const Access := preload("res://scripts/battle/battle_access.gd")
const TerrainTip := preload("res://scripts/battle/battle_terrain_tip.gd")


func _init() -> void:
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
	_check_floor()
	_check_marks()
	_check_terrain_tip()
	_check_colorblind(settings)
	finish()


func _check_floor() -> void:
	var theme := load("res://scenes/ui/parchment_theme.tres") as Theme
	check(theme != null, "G4: thème parchemin chargé")
	check_eq(theme.get_font_size("font_size", "Caption") >= MIN_FONT, true, "G4: Caption du thème ≥ %d" % MIN_FONT)
	for variation in [UiType.TITLE, UiType.HEADING, UiType.BODY, UiType.CAPTION]:
		check(theme.get_font_size("font_size", variation) >= MIN_FONT, "G4: %s ≥ %d dans le thème" % [variation, MIN_FONT])
		check(UiType.size(variation) >= MIN_FONT, "G4: UiType.%s ≥ %d" % [variation, MIN_FONT])
	check(theme.default_font_size >= MIN_FONT, "G4: taille par défaut du thème ≥ %d" % MIN_FONT)


func _check_marks() -> void:
	check_eq(RichTooltip.mark(1), "▲ ", "G3: bon = ▲")
	check_eq(RichTooltip.mark(-1), "▼ ", "G3: mauvais = ▼")
	check_eq(RichTooltip.mark(0), "", "G3: neutre sans marque")
	var bbcode := RichTooltip.to_bbcode({"title": "T", "effects": [{"sign": 1, "text": "x"}, {"sign": -1, "text": "y"}]})
	check(bbcode.contains("▲") and bbcode.contains("▼"), "G3: effets de l'infobulle marqués ▲/▼")


func _check_terrain_tip() -> void:
	var decor := {"label": "verger", "cover_pct": 20, "foot_speed_pct": -15, "horse_speed_pct": -15, "defense_pct": 0, "breaks_charge": false}
	var text := TerrainTip.text_for(decor)
	check(text.begins_with("Verger — "), "G1: étiquette « Verger — … » (%s)" % text)
	check(text.contains("couvert ▲ +20 %") and text.contains("vitesse ▼ −15 %"), "G1: valeurs signées (%s)" % text)
	check_eq(TerrainTip.text_for({}), "", "G1: pas de décor, pas d'étiquette")


func _check_colorblind(settings: Node) -> void:
	if settings == null:
		return
	settings.call("set_value", Accessibility.KEY_COLORBLIND, false, false)
	var livery := Color(0.7, 0.2, 0.2)
	check_eq(Access.side_color(livery, false), livery, "G2: sans daltonisme, livrée inchangée")
	settings.call("set_value", Accessibility.KEY_COLORBLIND, true, false)
	check_eq(Access.side_color(livery, true), Access.FRIEND, "G2: ami = bleu Okabe-Ito")
	check_eq(Access.side_color(livery, false), Access.ENEMY, "G2: ennemi = vermillon")
	check(Access.FRIEND.get_luminance() != Access.ENEMY.get_luminance(), "G2: ami/ennemi de luminances distinctes")
	settings.call("set_value", Accessibility.KEY_COLORBLIND, false, false)
