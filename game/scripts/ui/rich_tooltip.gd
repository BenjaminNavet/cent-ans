class_name RichTooltip
extends RefCounted

## Infobulles riches au style parchemin (F2). `make_panel(bbcode)` construit le contrôle
## renvoyé par `_make_custom_tooltip` des classes `RichButton`, `RichPanel`, `IconChip` ;
## les fonctions `unit`, `building`, `technology`, `resource`, `gauge`, `trait_tip`,
## `skill`, `population_class` produisent le BBCode (icône, coût, entretien, effets,
## prérequis…). Valeurs dynamiques (coût effectif, disponibilité, refus) : dictionnaires
## de `CampaignSim` passés en `live` ; définitions : `GameCatalog` (données `data/`).
## Aucune règle de jeu ici : seulement de la mise en forme.
##
## Autoloads obtenus par `/root/...` (le smoke `--script` est compilé avant eux).

const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const WIDTH := 330.0
const INK := Color(0.22, 0.14, 0.07)
const RED := "#8b1a1a"
const GREEN := "#2a6a2a"
const MUTED := "#6b5a40"
const POUND := Money.SYMBOL

const EFFECT_LABELS := {
	"unrest": "Mécontentement", "prestige": "Prestige", "army_morale": "Moral des armées",
	"piety": "Piété", "health": "Santé", "wealth": "Richesse", "trade_income": "Revenus du commerce",
	"research_points": "Points de recherche", "research_civil": "Recherche civile",
	"loyalty": "Loyauté", "army_ranged": "Tir", "army_armor": "Armure", "army_melee": "Mêlée",
	"tax_income": "Impôts", "production": "Production", "intrigue": "Intrigue",
	"diplomacy": "Diplomatie", "battle_charge": "Charge en bataille",
	"battle_ranged": "Tir en bataille", "battle_defense": "Défense en bataille",
	"siege_resistance": "Résistance aux sièges", "siege_speed": "Vitesse de siège",
	"army_experience": "Expérience des troupes", "recruit_slots": "Places de recrutement",
	"recruit_cost": "Coût de recrutement", "fortification_level": "Fortifications",
	"growth": "Croissance", "movement": "Mouvement", "goods_satisfaction": "Satisfaction en biens",
	"garrison": "Garnison", "fertility": "Fécondité", "construction_speed": "Vitesse de construction",
	"attrition_resistance": "Résistance à l'attrition", "supply": "Ravitaillement",
	"army_upkeep": "Entretien des armées",
	# H9 : médecine et table.
	"plague_resistance": "Résistance à la peste", "wound_recovery": "Soin des blessés",
	"diet_health": "Santé tirée des régimes", "research_military": "Recherche militaire",
}
## Branches de technologies (H9 : médecine) ; forme adjectivale pour les sous-titres.
const TECH_BRANCH_LABELS := {"military": "militaire", "civil": "civile", "medicine": "médecine"}
## H9 : terrains exigés par un régime (`requirements.terrains`).
const TERRAIN_LABELS := {
	"plains": "plaines", "hills": "collines", "mountains": "montagnes", "forest": "forêt",
	"marsh": "marais", "coast": "littoral", "highlands": "hautes terres", "bocage": "bocage",
}
## H9 : règles de Carême et d'hiver d'un régime (`lent_rule`, `winter_rule`), texte d'affichage.
const LENT_RULE_TEXTS := {
	"meat": "Carême : table grasse, −3 piété du souverain et +10 de mécontentement du clergé",
	"dairy": "Carême : laitages interdits, −3 piété du souverain et +10 de mécontentement du clergé",
	"fish": "Carême : table maigre, +2 piété du souverain",
}
const WINTER_RULE_TEXTS := {"fresh": "Denrées fraîches : coût ×1,5 en hiver"}
const STAT_LABELS := {
	"melee": "Mêlée", "ranged": "Tir", "range": "Portée", "armor": "Armure", "morale": "Moral",
	"speed": "Vitesse", "ammo": "Munitions", "charge": "Charge", "siege_attack": "Attaque de siège",
}
## Statistiques comparées à la moyenne des unités pour les forces/faiblesses.
const COMPARED_STATS := ["melee", "ranged", "armor", "morale", "speed", "charge", "siege_attack"]
const ABILITY_LABELS := {
	"stakes": "pieux plantés", "rain_penalty": "gêné par la pluie", "pavise": "pavois",
	"volley": "tir en volées", "wall_assault": "assaut des murailles", "wall_breach": "ouvre des brèches",
	"pike_square": "hérisson de piques", "skirmish": "escarmouche", "shield_wall": "mur de boucliers",
	"charge_lance": "charge à la lance", "dismount": "combat démonté",
}
const UNIT_CATEGORY_LABELS := {"infantry": "infanterie", "ranged": "tireurs", "cavalry": "cavalerie", "siege": "engin de siège"}
const BUILDING_CATEGORY_LABELS := {
	"production": "production", "commerce": "commerce", "military": "militaire",
	"religious": "religieux", "sanitary": "sanitaire", "fortification": "fortification",
}
const RESOURCE_CATEGORY_LABELS := {
	"food": "nourriture", "raw_material": "matière première", "manufactured": "produit manufacturé",
	"luxury": "luxe", "textile": "textile", "metal": "métal",
}
const CLASS_LABELS := {"peasants": "Paysans", "burghers": "Bourgeois", "clergy": "Clergé", "nobility": "Noblesse"}
const TRAIT_CATEGORY_LABELS := {
	"personality": "personnalité", "physical": "physique", "martial": "martial",
	"governance": "gouvernance", "acquired": "acquis",
}
const BRANCH_LABELS := {"command": "Commandement", "governance": "Gouvernance", "court": "Cour"}
const BRANCH_TEXTS := {
	"command": "Commandement : conduite des armées (moral, tir, charge, sièges, logistique).",
	"governance": "Gouvernance : administration des provinces (impôts, construction, ordre public).",
	"court": "Cour : diplomatie, intrigue, prestige et piété.",
}
const GAUGE_TEXTS := {
	"unrest": ["Mécontentement", "Tend vers le fardeau fiscal, la dévastation, le manque de biens et de santé, l'occupation étrangère, la religion différente et les troubles récents (prises, pillages, régence) ; la garnison et certains bâtiments l'apaisent. Au-delà de 75 pendant trois saisons : révolte (le compte repart ensuite de zéro) ; au-delà de 90 : la province passe aux rebelles."],
	"health": ["Santé", "Tend vers 50 + bâtiments sanitaires + satisfaction en biens, moins la surpopulation. Sous 50 la population décline ; sous 30, risque de peste."],
	"wealth": ["Richesse", "Tend vers la base de la classe + bâtiments de commerce, moins le fardeau fiscal et la dévastation."],
	"goods_satisfaction": ["Biens", "Satisfaction en biens : 40 + 10 par catégorie de biens accessible à la faction (ressources des provinces contrôlées et alliées) + marchés et foires."],
	"devastation": ["Dévastation", "Pillages et combats : freine la croissance (nulle au-delà de 50), la richesse et nourrit le mécontentement."],
	"population": ["Population", "Habitants de la province, toutes classes confondues ; croît avec la santé."],
	"morale": ["Moral", "Au plus bas, l'unité rompt et fuit le combat."],
	"supply": ["Ravitaillement", "Vivres de l'armée : baisse en territoire hostile ou dévasté, remonte en territoire ami."],
	"movement": ["Mouvement", "Points de mouvement restants ce tour."],
	"strength": ["Effectif", "Hommes présents / effectif complet de l'unité."],
}
const HUD_TEXTS := {
	"hud_treasury": ["Trésor", "Livres disponibles pour recruter, construire et entretenir armées et bâtiments. En dette, les troupes se débandent."],
	"hud_income": ["Solde", "Recettes de la saison (impôts, commerce, seigneuriage) moins l'entretien des armées, des bâtiments, de la Table et de l'administration : ce qui sera ajouté au trésor en fin de tour."],
	"hud_research": ["Recherche", "Technologie en cours ; clic : arbre des technologies."],
	"hud_court": ["Cour", "Personnages de la faction (touche C)."],
	"hud_technologies": ["Technologies", "Arbres militaire, civil et médecine (touche T)."],
	"hud_diplomacy": ["Diplomatie", "Relations, traités et religion (touche P)."],
	"hud_chronicle": ["Chronique", "Événements historiques et aléatoires en attente de décision."],
	"hud_end_turn": ["Fin du tour", "Termine la saison (Entrée)."],
	"hud_season_spring": ["Printemps", ""],
	"hud_season_summer": ["Été", ""],
	"hud_season_autumn": ["Automne", ""],
	"hud_season_winter": ["Hiver", ""],
}
const SEASON_WORDS := {"printemps": "spring", "été": "summer", "ete": "summer", "automne": "autumn", "hiver": "winter"}


static func icons() -> Node:
	var loop := Engine.get_main_loop() as SceneTree
	return loop.root.get_node_or_null("/root/IconLibrary") if loop != null else null


static func icon_bbcode(id: String, size: int = 18, category: String = "") -> String:
	var library := icons()
	return str(library.call("bbcode", id, size, category)) if library != null else ""


## Dernière infobulle construite (épinglage par `CodexBubbles`, touche T) et son BBCode.
static var last_panel: WeakRef = null
static var last_bbcode: String = ""


## Style parchemin commun aux infobulles et aux bulles du Codex.
static func panel_style() -> StyleBox:
	return HudStyle.note_box(6)


## Contrôle d'infobulle : panneau parchemin + texte BBCode (largeur fixe, hauteur ajustée).
## Le texte passe par `CodexText.format` (liens `[[…]]` et alias du Codex rubriqués). B1 : pied
## « T : maintenir ouverte » (la touche T verrouille l'infobulle en bulle du Codex).
static func make_panel(bbcode: String) -> Control:
	var panel := PanelContainer.new()
	if ResourceLoader.exists(THEME_PATH):
		panel.theme = load(THEME_PATH)
	panel.add_theme_stylebox_override("panel", panel_style())
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	panel.add_child(box)
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(WIDTH, 0)
	label.add_theme_color_override("default_color", INK)
	label.add_theme_font_size_override("normal_font_size", 14)
	label.add_theme_font_size_override("bold_font_size", 15)
	label.text = CodexText.format(bbcode, true)
	label.name = "Text"
	box.add_child(label)
	var footer := Label.new()
	footer.name = "Footer"
	footer.text = footer_text(title_entry(label.text) != "")
	footer.add_theme_font_size_override("font_size", 11)
	footer.add_theme_color_override("font_color", Color(MUTED))
	footer.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	box.add_child(footer)
	last_panel = weakref(panel)
	last_bbcode = label.text
	return panel


## Pied des infobulles riches : la fiche liée au titre se lit une fois l'infobulle verrouillée.
static func footer_text(has_entry: bool) -> String:
	return "T : maintenir ouverte" + (" · puis clic : lire la fiche" if has_entry else "")


## Fiche du Codex liée au titre gras (`[b][url=cdx:…]`, première ligne) d'une infobulle, vide sinon.
static func title_entry(bbcode: String) -> String:
	var first_line := bbcode.get_slice("\n", 0)
	var start := first_line.find("[b][url=" + CodexText.META_PREFIX)
	if start < 0:
		return ""
	start += ("[b][url=" + CodexText.META_PREFIX).length()
	var end := first_line.find("]", start)
	return first_line.substr(start, end - start) if end > start else ""


## B1 : nom d'une entité de jeu, lié à sa fiche du Codex (`entity` de la fiche) s'il y en a une.
static func entity_name(id: String, name: String) -> String:
	var codex := CodexText.store()
	var entry := str(codex.call("entry_for_entity", id)) if codex != null and id != "" else ""
	return CodexText.link(entry, name) if entry != "" else name


## Infobulle native actuellement affichée (dans sa fenêtre surgissante), sinon null.
static func visible_panel() -> Control:
	var panel: Control = last_panel.get_ref() if last_panel != null else null
	if panel == null or not panel.is_inside_tree() or not panel.is_visible_in_tree():
		return null
	var window := panel.get_window()
	var loop := Engine.get_main_loop() as SceneTree
	if window == null or (loop != null and window == loop.root) or not window.visible:
		return null
	return panel


static func thousands(value: int) -> String:
	return Money.digits(value)


static func _title(id: String, name: String, subtitle: String = "", category: String = "") -> String:
	var icon := icon_bbcode(id, 28, category)
	name = entity_name(id, name)
	var head := "%s [b]%s[/b]" % [icon, name] if icon != "" else "[b]%s[/b]" % name
	if subtitle != "":
		head += "  [color=%s][i]%s[/i][/color]" % [MUTED, subtitle]
	return head


static func _number(value: float) -> String:
	return str(int(value)) if is_equal_approx(value, roundf(value)) else str(snappedf(value, 0.01))


## « Santé +5 % (paysans) » : effet de `data/` (`effect`) ou de la simulation (`kind`).
static func effect_text(effect: Dictionary) -> String:
	var kind: String = str(effect.get("kind", effect.get("effect", "")))
	var label: String = str(EFFECT_LABELS.get(kind, kind.replace("_", " ")))
	var value := float(effect.get("value", 0))
	var suffix := " %" if str(effect.get("mode", "add")) == "percent" else ""
	var text := "%s %s%s%s" % [label, "+" if value >= 0 else "", _number(value), suffix]
	var qualifiers := PackedStringArray()
	var category: String = str(effect.get("unit_category", ""))
	if category != "":
		qualifiers.append(str(UNIT_CATEGORY_LABELS.get(category, category)))
	var class_id: String = str(effect.get("class", ""))
	if class_id != "":
		qualifiers.append(str(CLASS_LABELS.get(class_id, class_id)).to_lower())
	if not qualifiers.is_empty():
		text += " (%s)" % ", ".join(qualifiers)
	return text


static func _effects_block(effects: Array, title: String = "Effets") -> String:
	var parts := PackedStringArray()
	for effect in effects:
		if effect is Dictionary:
			parts.append(effect_text(effect))
	return "%s : %s" % [title, " · ".join(parts)] if not parts.is_empty() else ""


## Coût `{money, resources{res_id: n}}` → « 800 ₶ + [fer] 2 ».
static func cost_text(cost: Variant) -> String:
	if cost is int or cost is float:
		return "%s %s" % [thousands(int(cost)), POUND]
	if not (cost is Dictionary):
		return ""
	var parts := PackedStringArray()
	if cost.has("money"):
		parts.append("%s %s" % [thousands(int(cost["money"])), POUND])
	var resources: Dictionary = cost.get("resources", {})
	for res_id in resources:
		parts.append("%s %s %d" % [icon_bbcode(str(res_id), 14), GameCatalog.display_name(str(res_id)), int(resources[res_id])])
	return " + ".join(parts)


static func _description(definition: Dictionary) -> String:
	var description: String = str(definition.get("description", ""))
	return "[color=%s][i]%s[/i][/color]" % [MUTED, description] if description != "" else ""


static func _unavailable(live: Dictionary) -> String:
	if live.is_empty() or bool(live.get("available", true)):
		return ""
	return "[color=%s]Indisponible : %s[/color]" % [RED, str(live.get("reason", "conditions non remplies"))]


static func _join(lines: Array) -> String:
	var kept := PackedStringArray()
	for line in lines:
		if str(line) != "":
			kept.append(str(line))
	return "\n".join(kept)


# --- Unités -------------------------------------------------------------------------------


## Forces et faiblesses : statistiques ≥ 130 % ou ≤ 70 % de la moyenne des types d'unités.
## Lot UR1 : époque de recrutement d'un type d'unité (`available_from` / `available_until`),
## « » si l'unité est de tout temps.
static func unit_period(definition: Dictionary) -> String:
	var from := int(definition.get("available_from", 0))
	var until := int(definition.get("available_until", 0))
	if from > 0 and until > 0:
		return "%d-%d" % [from, until]
	if from > 0:
		return "à partir de %d" % from
	if until > 0:
		return "jusqu'en %d" % until
	return ""


static func strengths_weaknesses(definition: Dictionary) -> Array:
	var strengths := PackedStringArray()
	var weaknesses := PackedStringArray()
	var stats: Dictionary = definition.get("stats", {})
	for stat in COMPARED_STATS:
		if not stats.has(stat):
			continue
		var average := GameCatalog.unit_stat_average(stat)
		var value := float(stats[stat])
		if average <= 0.0:
			continue
		if value >= average * 1.3:
			strengths.append(str(STAT_LABELS[stat]).to_lower())
		elif value <= average * 0.7 and stat != "siege_attack" and stat != "charge":
			weaknesses.append("ne tire pas" if stat == "ranged" and value <= 0.0 else str(STAT_LABELS[stat]).to_lower())
	for ability in definition.get("abilities", []):
		if str(ability) == "rain_penalty":
			weaknesses.append(str(ABILITY_LABELS[ability]))
	return [strengths, weaknesses]


## `live` : ligne de `get_recruitable` (`cost`, `upkeep`, `available`, `reason`) ou unité
## d'armée (`strength`, `max_strength`, `morale`) ; vide pour la seule définition.
static func unit(unit_type: String, live: Dictionary = {}) -> String:
	var definition := GameCatalog.unit_type(unit_type)
	var name: String = str(live.get("name", ""))
	if name == "":
		name = GameCatalog.display_name(unit_type)
	var category: String = str(definition.get("category", ""))
	var subtitle := str(UNIT_CATEGORY_LABELS.get(category, category))
	if definition.has("soldiers"):
		subtitle += ", %d hommes" % int(definition["soldiers"])
	var lines: Array = [_title(unit_type, name, subtitle, "unit")]
	if live.has("strength"):
		lines.append("Effectif : %d / %d · moral %d" % [int(live.get("strength", 0)), int(live.get("max_strength", 0)), int(live.get("morale", 0))])
	var cost_line := PackedStringArray()
	if live.has("cost"):
		cost_line.append("Coût : %s %s" % [thousands(int(live["cost"])), POUND])
	elif definition.has("cost"):
		cost_line.append("Coût : " + cost_text(definition["cost"]))
	var upkeep := int(live.get("upkeep", definition.get("upkeep", -1)))
	if upkeep >= 0:
		cost_line.append("Entretien : %s %s / saison" % [thousands(upkeep), POUND])
	if definition.has("recruit_time_turns"):
		cost_line.append("Levée : %s" % FrText.count(int(definition["recruit_time_turns"]), "tour"))
	lines.append(" · ".join(cost_line))
	var stats: Dictionary = definition.get("stats", {})
	var stat_parts := PackedStringArray()
	for stat in ["melee", "ranged", "range", "armor", "morale", "speed", "charge", "siege_attack", "ammo"]:
		if stats.has(stat) and (float(stats[stat]) > 0.0 or stat in ["melee", "armor", "morale"]):
			stat_parts.append("%s %s" % [STAT_LABELS[stat], _number(float(stats[stat]))])
	if not stat_parts.is_empty():
		lines.append(" · ".join(stat_parts))
	if not definition.is_empty():
		var sw := strengths_weaknesses(definition)
		if not (sw[0] as PackedStringArray).is_empty():
			lines.append("[color=%s]Forces : %s[/color]" % [GREEN, ", ".join(sw[0])])
		if not (sw[1] as PackedStringArray).is_empty():
			lines.append("[color=%s]Faiblesses : %s[/color]" % [RED, ", ".join(sw[1])])
	var abilities := PackedStringArray()
	for ability in definition.get("abilities", []):
		if str(ability) != "rain_penalty":
			abilities.append(str(ABILITY_LABELS.get(ability, ability)))
	if not abilities.is_empty():
		lines.append("Capacités : " + ", ".join(abilities))
	var requires := PackedStringArray()
	if str(definition.get("required_technology", "")) != "":
		requires.append("%s %s" % [icon_bbcode(str(definition["required_technology"]), 14), GameCatalog.display_name(str(definition["required_technology"]))])
	# B7c : le bâtiment requis se lit dans `enables_units` des bâtiments (seule source, lue par le core).
	var enablers := PackedStringArray()
	for building_id in enabling_buildings(str(definition.get("id", ""))):
		enablers.append("%s %s" % [icon_bbcode(building_id, 14), GameCatalog.display_name(building_id)])
	if not enablers.is_empty():
		requires.append(" ou ".join(enablers) + " (ou supérieur)")
	if not requires.is_empty():
		lines.append("Requiert : " + ", ".join(requires))
	var period := unit_period(definition)
	if period != "":
		lines.append("Époque : " + period)
	if str(definition.get("source_class", "")) != "":
		lines.append("Recrutés parmi : %s" % str(CLASS_LABELS.get(definition["source_class"], definition["source_class"])).to_lower())
	lines.append(_description(definition))
	lines.append(_unavailable(live))
	return _join(lines)


# --- Bâtiments ----------------------------------------------------------------------------


## B7c : bâtiments dont `enables_units` contient `unit_id` (triés).
static func enabling_buildings(unit_id: String) -> Array:
	var found: Array = []
	var buildings := GameCatalog.definitions("buildings")
	for building_id in buildings:
		if unit_id in (buildings[building_id] as Dictionary).get("enables_units", []):
			found.append(str(building_id))
	found.sort()
	return found


## B7c : vrai si un autre bâtiment s'élève à la place de `building_id` (un prérequis accepte alors
## l'amélioration).
static func has_upgrade(building_id: String) -> bool:
	var buildings := GameCatalog.definitions("buildings")
	for other in buildings:
		if str((buildings[other] as Dictionary).get("upgrades_from", "")) == building_id:
			return true
	return false


## `live` : ligne de `buildable` (`cost`, `turns`, `available`, `reason`) ou bâtiment
## construit (`upkeep`).
static func building(building_id: String, live: Dictionary = {}) -> String:
	var definition := GameCatalog.building(building_id)
	var name: String = str(live.get("name", ""))
	if name == "":
		name = GameCatalog.display_name(building_id)
	var category: String = str(definition.get("category", live.get("category", "")))
	var subtitle := str(BUILDING_CATEGORY_LABELS.get(category, category))
	if definition.has("tier"):
		subtitle += ", rang %d" % int(definition["tier"])
	var lines: Array = [_title(building_id, name, subtitle, "building")]
	var cost_line := PackedStringArray()
	if live.has("cost"):
		cost_line.append("Coût : %s %s" % [thousands(int(live["cost"])), POUND])
	elif definition.has("cost"):
		cost_line.append("Coût : " + cost_text(definition["cost"]))
	var turns := int(live.get("turns", definition.get("build_time_turns", 0)))
	if turns > 0:
		cost_line.append("Durée : %s" % FrText.count(turns, "tour"))
	var upkeep := int(live.get("upkeep", definition.get("upkeep", 0)))
	cost_line.append("Entretien : %s %s / saison" % [thousands(upkeep), POUND])
	lines.append(" · ".join(cost_line))
	if live.has("cost") and definition.has("cost") and (definition["cost"] as Dictionary).has("resources"):
		lines.append("Matériaux : " + cost_text({"resources": definition["cost"]["resources"]}))
	# B7c : matériaux tirés des provinces productrices ; le manque est importé et compté dans le coût.
	if int(live.get("import_cost", 0)) > 0:
		lines.append("[color=%s]Dont importation : %s %s (%s manquant)[/color]" % [RED, thousands(int(live["import_cost"])), POUND, cost_text({"resources": live.get("imported", {})})])
	elif definition.has("cost") and (definition["cost"] as Dictionary).has("resources"):
		lines.append("Matériaux tirés de vos provinces productrices, sinon importés (prix de base × 100).")
	lines.append(_effects_block(definition.get("effects", [])))
	var units := PackedStringArray()
	for unit_id in definition.get("enables_units", []):
		units.append("%s %s" % [icon_bbcode(str(unit_id), 14), GameCatalog.display_name(str(unit_id))])
	if not units.is_empty():
		lines.append("Permet de lever : " + ", ".join(units))
	var requires := PackedStringArray()
	for key in ["upgrades_from", "required_building", "required_technology", "required_resource"]:
		var value: String = str(definition.get(key, ""))
		if value != "":
			var note := ""
			if key == "upgrades_from":
				note = " (amélioration : le remplace et garde ses effets)"
			elif key == "required_building" and has_upgrade(value):
				note = " ou supérieur"
			requires.append("%s %s%s" % [icon_bbcode(value, 14), GameCatalog.display_name(value), note])
	if bool(definition.get("requires_coastal", false)):
		requires.append("province côtière")
	if bool(definition.get("requires_river", false)):
		requires.append("rivière")
	if not requires.is_empty():
		lines.append("Prérequis : " + ", ".join(requires))
	lines.append(_description(definition))
	lines.append(_unavailable(live))
	return _join(lines)


# --- Technologies -------------------------------------------------------------------------


const TECH_STATE_LABELS := {"known": "Acquise", "researching": "En cours", "available": "Disponible", "locked": "Verrouillée"}


## `node` : entrée de `get_tech_tree` (effets, coûts effectif et de base, prérequis, date).
static func technology(node: Dictionary) -> String:
	var id: String = str(node.get("id", ""))
	var state: String = str(node.get("state", ""))
	var branch: String = str(node.get("branch", ""))
	var subtitle := "%s, rang %d — %s" % [TECH_BRANCH_LABELS.get(branch, branch), int(node.get("tier", 1)), TECH_STATE_LABELS.get(state, state)]
	var lines: Array = [_title(id, str(node.get("name", id)), subtitle, "technology")]
	var cost := int(node.get("cost", 0))
	var effective := int(node.get("effective_cost", cost))
	var cost_line := "Coût : %d points" % effective
	if effective > cost:
		cost_line += " [color=%s](%d + 25 %% : en avance sur son temps)[/color]" % [RED, cost]
	if int(node.get("progress", 0)) > 0 and state != "known":
		cost_line += " · %d / %d" % [int(node.get("progress", 0)), effective]
	lines.append(cost_line)
	var year := int(node.get("historical_year", 0))
	if year > 0:
		lines.append("Date historique : %s%d" % ["vers " if bool(node.get("historical_uncertain", false)) else "", year])
	lines.append(_effects_block(node.get("effects", [])))
	var unlocks: Dictionary = node.get("unlocks", {})
	var unlocked := PackedStringArray()
	for unit_id in unlocks.get("units", []):
		unlocked.append("%s %s" % [icon_bbcode(str(unit_id), 14), GameCatalog.display_name(str(unit_id))])
	for building_id in unlocks.get("buildings", []):
		unlocked.append("%s %s" % [icon_bbcode(str(building_id), 14), GameCatalog.display_name(str(building_id))])
	if not unlocked.is_empty():
		lines.append("Débloque : " + ", ".join(unlocked))
	var prerequisites := PackedStringArray()
	for prereq in node.get("prerequisites", []):
		prerequisites.append("%s %s" % [icon_bbcode(str(prereq), 14), GameCatalog.display_name(str(prereq))])
	if not prerequisites.is_empty():
		lines.append("Prérequis : " + ", ".join(prerequisites))
	lines.append(herbs_line(node.get("herbs", [])))
	lines.append(_description(node))
	var note: String = str(node.get("historical_note", ""))
	if note != "":
		lines.append("[color=%s][i]%s[/i][/color]" % [MUTED, note])
	return _join(lines)


## H9 : « Plantes : sauge, rue… » en liens du Codex (nom tiré de l'id si la fiche manque).
static func herbs_line(herbs: Variant) -> String:
	var parts := PackedStringArray()
	if herbs is Array or herbs is PackedStringArray:
		for herb in herbs:
			var id := str(herb)
			parts.append(CodexText.link(id, "" if _codex_has(id) else herb_name(id)))
	return "Plantes : " + ", ".join(parts) if not parts.is_empty() else ""


## Nom lisible d'une plante sans fiche : `cdx_reine_des_pres` → « reine des pres ».
static func herb_name(id: String) -> String:
	return id.trim_prefix("cdx_").replace("_", " ")


static func _codex_has(id: String) -> bool:
	var codex := CodexText.store()
	return codex != null and bool(codex.call("has_entry", id))


# --- Régimes (H9, La Table) ------------------------------------------------------------------


## `option` : entrée de `get_diet_options` (coût effectif, effets, conditions, `reasons`).
static func diet(option: Dictionary) -> String:
	var id: String = str(option.get("id", ""))
	var state := "régime actuel" if bool(option.get("current", false)) else ("disponible" if bool(option.get("available", false)) else "indisponible")
	var lines: Array = [_title(id, str(option.get("name", id)), state, "resource")]
	var cost := int(option.get("cost", 0))
	var cost_line := "Coût : %s %s / saison" % [thousands(cost), POUND] if cost > 0 else "Coût : aucun"
	var per_thousand := float(option.get("cost_per_thousand", 0))
	if per_thousand > 0.0:
		cost_line += " [color=%s](%s %s pour 1 000 habitants)[/color]" % [MUTED, str(snappedf(per_thousand, 0.01)).replace(".", ","), POUND]
	lines.append(cost_line)
	lines.append(_effects_block(option.get("effects", [])))
	lines.append(str(LENT_RULE_TEXTS.get(str(option.get("lent_rule", "none")), "")))
	lines.append(str(WINTER_RULE_TEXTS.get(str(option.get("winter_rule", "none")), "")))
	lines.append(_diet_requirements(option.get("requirements", {})))
	var reasons := PackedStringArray()
	for reason in option.get("reasons", []):
		reasons.append(str(reason))
	if not reasons.is_empty():
		lines.append("[color=%s]Manque : %s[/color]" % [RED, " ; ".join(reasons)])
	lines.append(_description(option))
	return _join(lines)


static func _diet_requirements(requirements: Variant) -> String:
	if not (requirements is Dictionary):
		return ""
	var parts := PackedStringArray()
	for res_id in requirements.get("resources", []):
		parts.append("%s %s" % [icon_bbcode(str(res_id), 14), GameCatalog.display_name(str(res_id))])
	if bool(requirements.get("coastal", false)):
		parts.append("province côtière")
	var technology: String = str(requirements.get("technology", ""))
	if technology != "":
		parts.append("%s %s" % [icon_bbcode(technology, 14), GameCatalog.display_name(technology)])
	var buildings := PackedStringArray()
	for building_id in requirements.get("any_building", []):
		buildings.append("%s %s" % [icon_bbcode(str(building_id), 14), GameCatalog.display_name(str(building_id))])
	if not buildings.is_empty():
		parts.append(" ou ".join(buildings))
	var terrains := PackedStringArray()
	for terrain in requirements.get("terrains", []):
		terrains.append(str(TERRAIN_LABELS.get(str(terrain), str(terrain))))
	if not terrains.is_empty():
		parts.append("terrain : " + ", ".join(terrains))
	return "Conditions : " + " · ".join(parts) if not parts.is_empty() else ""


# --- Ressources, classes, jauges ------------------------------------------------------------


static func resource(resource_id: String, stock: int = -1) -> String:
	var definition := GameCatalog.resource(resource_id)
	var category: String = str(definition.get("category", ""))
	var lines: Array = [_title(resource_id, GameCatalog.display_name(resource_id), str(RESOURCE_CATEGORY_LABELS.get(category, category)), "resource")]
	if stock >= 0:
		lines.append("Quantité accessible : %d" % stock)
	if definition.has("base_price"):
		lines.append("Prix de base : %s %s" % [_number(float(definition["base_price"])), POUND])
	var classes := PackedStringArray()
	for class_id in definition.get("satisfies_classes", []):
		classes.append("%s %s" % [icon_bbcode("class_" + str(class_id), 14), str(CLASS_LABELS.get(class_id, class_id)).to_lower()])
	if not classes.is_empty():
		lines.append("Satisfait : " + ", ".join(classes))
	elif definition.has("satisfies_classes"):
		lines.append("Satisfait : aucune classe (matériau)")
	lines.append(_description(definition))
	return _join(lines)


static func population_class(class_id: String, data: Dictionary = {}) -> String:
	var lines: Array = [_title("class_" + class_id, str(CLASS_LABELS.get(class_id, class_id)), "", "class")]
	if data.has("count"):
		lines.append("Population : %s" % thousands(int(data["count"])))
	var gauges := PackedStringArray()
	for key in ["unrest", "health", "wealth", "goods_satisfaction"]:
		if data.has(key):
			gauges.append("%s %s %d" % [icon_bbcode("gauge_" + key, 14), GAUGE_TEXTS[key][0], int(data[key])])
	if not gauges.is_empty():
		lines.append(" · ".join(gauges))
	return _join(lines)


static func gauge(key: String, value: float = -1.0) -> String:
	var spec: Array = GAUGE_TEXTS.get(key, [key.capitalize(), ""])
	var head := _title("gauge_" + key, str(spec[0]), "%d / 100" % int(round(value)) if value >= 0.0 else "", "gauge")
	return _join([head, str(spec[1])])


static func hud(id: String, extra: String = "") -> String:
	var spec: Array = HUD_TEXTS.get(id, [id.trim_prefix("hud_").capitalize(), ""])
	return _join([_title(id, str(spec[0]), "", "hud"), str(spec[1]), extra])


## Saison (`spring`…) d'un libellé de date « Automne 1339 », "" si inconnue.
static func season_of(date_label: String) -> String:
	var first := date_label.strip_edges().split(" ", false)
	if first.is_empty():
		return ""
	return str(SEASON_WORDS.get(first[0].to_lower(), ""))


# --- Personnages ------------------------------------------------------------------------------


## `entry` : trait de `get_character` (`id`, `name`, `category`, `description`).
static func trait_tip(entry: Dictionary) -> String:
	var id: String = str(entry.get("id", ""))
	var definition := GameCatalog.trait_definition(id)
	var category: String = str(entry.get("category", definition.get("category", "")))
	var name: String = str(entry.get("name", GameCatalog.display_name(id)))
	var lines: Array = ["%s [b]%s[/b]  [color=%s][i]%s[/i][/color]" % [icon_bbcode("trait_category_" + category, 28, "trait"), name, MUTED, TRAIT_CATEGORY_LABELS.get(category, category)]]
	lines.append(_effects_block(definition.get("effects", [])))
	var opposites := PackedStringArray()
	for opposite in definition.get("opposites", entry.get("opposites", [])):
		opposites.append(GameCatalog.display_name(str(opposite)))
	if not opposites.is_empty():
		lines.append("Incompatible avec : " + ", ".join(opposites))
	var description: String = str(entry.get("description", definition.get("description", "")))
	if description != "":
		lines.append("[color=%s][i]%s[/i][/color]" % [MUTED, description])
	return _join(lines)


## `node` : entrée de `get_skill_tree` ; `state` : « Appris », « Disponible »…
static func skill(node: Dictionary, state: String = "") -> String:
	var branch: String = str(node.get("branch", ""))
	var name: String = str(node.get("name", node.get("id", "")))
	var head := "%s [b]%s[/b]  [color=%s][i]%s, rang %d%s[/i][/color]" % [
		icon_bbcode("branch_" + branch, 28, "branch"), name, MUTED,
		BRANCH_LABELS.get(branch, branch), int(node.get("tier", 1)), " — " + state if state != "" else ""]
	var lines: Array = [head, "Coût : %s de compétence" % FrText.count(int(node.get("cost", 0)), "point")]
	lines.append(_effects_block(node.get("effects", [])))
	var prerequisites := PackedStringArray()
	for prereq in node.get("prerequisites", []):
		prerequisites.append(GameCatalog.display_name(str(prereq)))
	if not prerequisites.is_empty():
		lines.append("Prérequis : " + ", ".join(prerequisites))
	lines.append(_description(node))
	return _join(lines)


static func branch(branch_id: String, value: int = -1) -> String:
	var head := _title("branch_" + branch_id, str(BRANCH_LABELS.get(branch_id, branch_id)), "niveau %d" % value if value >= 0 else "", "branch")
	return _join([head, str(BRANCH_TEXTS.get(branch_id, ""))])


# --- Monnaie, rançons, chevalerie (H11) -------------------------------------------------------


static func _signed_pounds(value: int) -> String:
	return "%s%s %s" % ["+" if value > 0 else "", thousands(value), POUND]


## `option` : entrée de `get_coinage().options` ; `changed_this_year` : déjà changée cette année.
static func coinage(option: Dictionary, changed_this_year: bool = false) -> String:
	var current := bool(option.get("current", false))
	var lines: Array = [_title("hud_treasury", str(option.get("label", option.get("level", ""))), "monnaie actuelle" if current else "", "hud")]
	var seigniorage := int(option.get("seigniorage", 0))
	var recoinage := int(option.get("recoinage", 0))
	lines.append("Seigneuriage : [color=%s]%s / saison[/color]" % [GREEN if seigniorage > 0 else MUTED, _signed_pounds(seigniorage)])
	if recoinage > 0:
		lines.append("Refonte des espèces : [color=%s]−%s %s / saison[/color] (administration)" % [RED, thousands(recoinage), POUND])
	var inflation := int(option.get("inflation", 0))
	var deflation := int(option.get("deflation", 0))
	if inflation > 0:
		lines.append("Prix : [color=%s]+%d / saison[/color] (recrutement, entretien et constructions renchérissent)" % [RED, inflation])
	elif deflation > 0:
		lines.append("Prix : [color=%s]−%d / saison[/color] (jusqu'aux prix de 1337)" % [GREEN, deflation])
	else:
		lines.append("Prix : stables")
	var unrest := float(option.get("burgher_unrest", 0.0))
	if not is_zero_approx(unrest):
		lines.append("Mécontentement des bourgeois : [color=%s]%s%s[/color] (cible)" % [RED if unrest > 0 else GREEN, "+" if unrest > 0 else "", _number(unrest)])
	var prestige := int(option.get("prestige", 0))
	if prestige != 0:
		lines.append("Prestige du souverain : [color=%s]%s%d / saison[/color]" % [GREEN if prestige > 0 else RED, "+" if prestige > 0 else "", prestige])
	lines.append("[color=%s][i]Un seul changement de monnaie par année civile.[/i][/color]" % MUTED)
	if changed_this_year and not current:
		lines.append("[color=%s]Refusé cette année : la monnaie a déjà été changée ; prochain changement possible l'an prochain.[/color]" % RED)
	return _join(lines)


## `option` : entrée de `get_chivalric_orders().options`.
static func chivalric_order(option: Dictionary) -> String:
	var lines: Array = [_title("hud_court", str(option.get("name", option.get("id", ""))), "disponible" if bool(option.get("available", false)) else "indisponible", "hud")]
	lines.append("Coût : %s %s · prestige requis : %d" % [thousands(int(option.get("cost", 0))), POUND, int(option.get("prestige_required", 0))])
	lines.append("Membres : %d (historiquement : %s)" % [int(option.get("members", 0)), str(option.get("historical_members", "—"))])
	lines.append("Membres : loyauté +%d, moral des armées qu'ils mènent +%d" % [int(option.get("member_loyalty", 0)), int(option.get("member_morale", 0))])
	lines.append("Souverain : prestige +%d à la fondation, +%d par an" % [int(option.get("founder_prestige", 0)), int(option.get("yearly_prestige", 0))])
	if int(option.get("min_year", 0)) > 0:
		lines.append("Fondation possible dès %d" % int(option.get("min_year", 0)))
	var reason := str(option.get("reason", ""))
	if reason != "" and not bool(option.get("available", false)):
		lines.append("[color=%s]Refus : %s[/color]" % [RED, reason])
	lines.append(_description(option))
	return _join(lines)
