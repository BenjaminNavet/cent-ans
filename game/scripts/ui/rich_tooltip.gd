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

## Branches de technologies (H9 : médecine) ; forme adjectivale pour les sous-titres.
const TECH_BRANCH_LABELS := {"military": "militaire", "civil": "civile", "medicine": "médecine"}
## H9 : terrains exigés par un régime (`requirements.terrains`).
const TERRAIN_LABELS := {
	"plains": "plaines", "hills": "collines", "mountains": "montagnes", "forest": "forêt",
	"marsh": "marais", "coast": "littoral", "highlands": "hautes terres", "bocage": "bocage",
	"heath": "lande", "steppe": "steppe", "desert": "désert",
}
## H9 : règles de Carême et d'hiver d'un régime (`lent_rule`, `winter_rule`), texte d'affichage.
const LENT_RULE_TEXTS := {
	"meat": "Carême : table grasse, −{rule.lent_piety_penalty} piété du souverain et +{rule.lent_clergy_unrest} de mécontentement du clergé",
	"dairy": "Carême : laitages interdits, −{rule.lent_piety_penalty} piété du souverain et +{rule.lent_clergy_unrest} de mécontentement du clergé",
	"fish": "Carême : table maigre, +{rule.lent_fish_piety} piété du souverain",
}
const WINTER_RULE_TEXTS := {"fresh": "Denrées fraîches : coût ×{rule.winter_fresh_cost_factor} en hiver"}
## Statistiques comparées à la moyenne des unités pour les forces/faiblesses.
const COMPARED_STATS := ["melee", "ranged", "armor", "morale", "speed", "charge", "siege_attack"]
const SEASON_WORDS := {"printemps": "spring", "été": "summer", "ete": "summer", "automne": "autumn", "hiver": "winter"}


static func icons() -> Node:
	var loop := Engine.get_main_loop() as SceneTree
	return loop.root.get_node_or_null("/root/IconLibrary") if loop != null else null


static func icon_bbcode(id: String, size: int = 18, category: String = "") -> String:
	var library := icons()
	return str(library.call("bbcode", id, size, category)) if library != null else ""


# --- Libellés d'affichage (IB2, ADR 0109) --------------------------------------------------
# `EFFECT_LABELS`, `STAT_LABELS`, `GAUGE_TEXTS`, `HUD_TEXTS`, et les tables de catégories,
# capacités, classes et branches vivaient ici en dur ; elles sont maintenant dans
# `data/ui/tooltips.json` (blocs `effects`, `stats`, `gauges`, `hud`, `categories`, `abilities`,
# `classes`, `branches`), lu avec repli par `texts()`. Aucune règle de jeu : uniquement le libellé
# français affiché pour une clé de `data/` ou du core.


## Libellé de `key` dans le bloc `block` de `tooltips.json` (`"label"` ou `"title"`), `fallback`
## si la clé est absente.
static func _label(block: String, key: String, fallback: String) -> String:
	var entry: Variant = (texts().get(block, {}) as Dictionary).get(key, null)
	if entry is Dictionary:
		return str(entry.get("label", entry.get("title", fallback)))
	return fallback


static func stat_label(key: String) -> String:
	return _label("stats", key, key)


static func unit_category_label(category: String) -> String:
	return _label("categories", category, category)


static func building_category_label(category: String) -> String:
	return _label("categories", category, category)


static func resource_category_label(category: String) -> String:
	return _label("categories", category, category)


static func trait_category_label(category: String) -> String:
	return _label("categories", category, category)


static func ability_label(id: String) -> String:
	return _label("abilities", id, id)


static func class_label(id: String) -> String:
	return _label("classes", id, id)


static func branch_label(id: String) -> String:
	return _label("branches", id, id)


## Texte descriptif d'une branche de compétence (`branches` de `tooltips.json`), "" si aucun.
static func branch_text(id: String) -> String:
	var entry: Variant = (texts().get("branches", {}) as Dictionary).get(id, null)
	return str(entry.get("body", "")) if entry is Dictionary else ""


## Vrai si `key` a un libellé d'effet dans `tooltips.json` (lien `ib:rule:` possible).
static func has_effect_label(key: String) -> bool:
	return (texts().get("effects", {}) as Dictionary).has(key)


## [titre, corps] d'une jauge (`gauges` de `tooltips.json`), capitalisation de `key` si absente.
static func gauge_entry(key: String) -> Array:
	var entry: Variant = (texts().get("gauges", {}) as Dictionary).get(key, null)
	if entry is Dictionary:
		return [str(entry.get("title", key.capitalize())), str(entry.get("body", ""))]
	return [key.capitalize(), ""]


## [titre, corps] d'une entrée du HUD (`hud` de `tooltips.json`), repli sur `id` sans préfixe.
static func hud_entry(id: String) -> Array:
	var entry: Variant = (texts().get("hud", {}) as Dictionary).get(id, null)
	if entry is Dictionary:
		return [str(entry.get("title", id)), str(entry.get("body", ""))]
	return [id.trim_prefix("hud_").capitalize(), ""]


## Dernière infobulle construite (épinglage par `CodexBubbles`, touche T) et son BBCode.
static var last_panel: WeakRef = null
static var last_bbcode: String = ""
## IB1 : spec de la dernière infobulle en sections (`TooltipView.build`), {} après `make_panel`.
static var last_spec: Dictionary = {}

## IB1 (ADR 0109) : préfixe des clés d'infobulle en sections portées par `tooltip_text`
## (« ib:<kind>:<id> », puis le BBCode de repli sur les lignes suivantes) ; le `live` de la clé
## est rangé en métadonnée `LIVE_META` du contrôle.
const KEY_PREFIX := "ib:"
const LIVE_META := &"ib_live"
## IB2 : script générique attaché par `attach_plain` aux contrôles natifs sans classe dédiée.
const PLAIN_HOST_SCRIPT := preload("res://scripts/ui/plain_tooltip_host.gd")


## Style parchemin commun aux infobulles et aux bulles du Codex.
static func panel_style() -> StyleBox:
	return HudStyle.note_box(6)


## Contrôle d'infobulle : panneau parchemin + texte BBCode (largeur fixe, hauteur ajustée).
## Le texte passe par `CodexText.format` (liens `[[…]]` et alias du Codex rubriqués). B1 : pied
## « T : maintenir ouverte » (la touche T verrouille l'infobulle en bulle du Codex).
static func make_panel(bbcode: String) -> Control:
	bbcode = fallback_of(bbcode)
	last_spec = {}
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
	# P2c : infobulle compacte — variation `Caption` (14 px, plancher de la bible § 12.2).
	UiType.apply(label, UiType.CAPTION)
	label.add_theme_font_size_override("bold_font_size", UiType.size(UiType.CAPTION))
	label.text = CodexText.format(bbcode, true)
	label.name = "Text"
	box.add_child(label)
	var footer := Label.new()
	footer.name = "Footer"
	footer.text = footer_text(title_entry(label.text) != "")
	UiType.apply(footer, UiType.CAPTION)
	footer.add_theme_color_override("font_color", Color(MUTED))
	footer.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	box.add_child(footer)
	last_panel = weakref(panel)
	last_bbcode = label.text
	return panel


## Pied des infobulles riches : la fiche liée au titre se lit une fois l'infobulle verrouillée.
static func footer_text(has_entry: bool) -> String:
	return "T : maintenir ouverte" + (" · puis clic : lire la fiche" if has_entry else "")


## IB1 : texte de `tooltip_text` d'une infobulle en sections — clé « ib:<kind>:<id> » sur la
## première ligne, BBCode de repli ensuite (lu par `make_panel` et par les tests de contenu).
static func tooltip_key(kind: String, id: String, fallback_bbcode: String) -> String:
	return "%s%s:%s\n%s" % [KEY_PREFIX, kind, id, fallback_bbcode]


## Clé « ib:<kind>:<id> » en tête de `text`, "" s'il n'en porte pas.
static func key_of(text: String) -> String:
	return text.get_slice("\n", 0) if text.begins_with(KEY_PREFIX) else ""


## `text` sans sa clé « ib: » éventuelle (le BBCode de repli).
static func fallback_of(text: String) -> String:
	if not text.begins_with(KEY_PREFIX):
		return text
	var cut := text.find("\n")
	return text.substr(cut + 1) if cut >= 0 else ""


## IB1 : pose l'infobulle en sections de `kind`/`id` sur `control` (clé + repli dans
## `tooltip_text`, `live` en métadonnée) ; `_make_custom_tooltip` la reconstruit par `panel_for`.
static func set_tooltip(control: Control, kind: String, id: String, live: Dictionary = {}) -> void:
	control.set_meta(LIVE_META, live)
	control.tooltip_text = tooltip_key(kind, id, to_bbcode(spec_for(KEY_PREFIX + kind + ":" + id, live)))


## IB1 : contrôle d'infobulle pour `_make_custom_tooltip(for_text)` de `owner` : rendu en
## sections (`TooltipView`, version courte) si le texte porte une clé « ib: », sinon `make_panel`.
static func panel_for(for_text: String, owner: Object = null) -> Control:
	var key := key_of(for_text)
	if key == "":
		return make_panel(for_text)
	var live: Dictionary = {}
	if owner != null and owner.has_meta(LIVE_META):
		live = owner.get_meta(LIVE_META)
	var spec := spec_for(key, live)
	if spec.is_empty():
		return make_panel(for_text)
	return TooltipView.build(spec, false)


## IB1 : spec d'infobulle (§ 2.1 de la spec IB) de la clé « ib:<kind>:<id> » ; {} si le type
## n'est pas (encore) décrit en sections. `live` : mêmes dictionnaires que les fonctions BBCode.
static func spec_for(key: String, live: Dictionary = {}) -> Dictionary:
	var parts := key.split(":", true, 2)
	if parts.size() < 3 or parts[0] + ":" != KEY_PREFIX:
		return {}
	var id: String = parts[2]
	match parts[1]:
		"unit":
			return unit_spec(id, live)
		"building":
			return building_spec(id, live)
		"technology":
			var node := live.duplicate() if not live.is_empty() else technology_node(id)
			node["id"] = id
			return technology_spec(node)
		"plain":
			return plain_spec(id, live)
	return {}


## Entrée de type `get_tech_tree` tirée de la seule définition (`data/technologies`), sans état
## ni coût effectif : bulles ouvertes depuis un lien, hors arbre des techniques.
static func technology_node(id: String) -> Dictionary:
	var definition := GameCatalog.technology(id)
	if definition.is_empty():
		return {}
	var node := definition.duplicate(true)
	node["id"] = id
	node["name"] = GameCatalog.display_name(id)
	var year: Variant = definition.get("historical_year", null)
	if year is Dictionary:
		node["historical_year"] = int(str(year.get("value", "0")))
		node["historical_uncertain"] = bool(year.get("uncertain", false))
		node["historical_note"] = str(year.get("note", ""))
	node["effective_cost"] = int(definition.get("cost", 0))
	return node


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


static func _title(id: String, name: String, subtitle: String = "", category: String = "", entity_id: String = "") -> String:
	var icon := icon_bbcode(id, 28, category)
	name = entity_name(entity_id if entity_id != "" else id, name)
	var head := "%s [b]%s[/b]" % [icon, name] if icon != "" else "[b]%s[/b]" % name
	if subtitle != "":
		head += "  [color=%s][i]%s[/i][/color]" % [MUTED, subtitle]
	return head


static func _number(value: float) -> String:
	return str(int(value)) if is_equal_approx(value, roundf(value)) else str(snappedf(value, 0.01))


## « Santé +5 % (paysans) » : effet de `data/` (`effect`) ou de la simulation (`kind`).
static func effect_text(effect: Dictionary) -> String:
	var kind: String = str(effect.get("kind", effect.get("effect", "")))
	return "%s %s%s" % [effect_label(kind), effect_value(effect), effect_qualifiers(effect)]


static func effect_label(kind: String) -> String:
	return _label("effects", kind, kind.replace("_", " "))


## « +5 % », « −3 » : valeur signée d'un effet.
static func effect_value(effect: Dictionary) -> String:
	var value := float(effect.get("value", 0))
	var suffix := " %" if str(effect.get("mode", "add")) == "percent" else ""
	return "%s%s%s" % ["+" if value >= 0 else "", _number(value), suffix]


## « (paysans) », « (cavalerie) » : portée d'un effet, "" sinon.
static func effect_qualifiers(effect: Dictionary) -> String:
	var text := ""
	var qualifiers := PackedStringArray()
	var category: String = str(effect.get("unit_category", ""))
	if category != "":
		qualifiers.append(unit_category_label(category))
	var class_id: String = str(effect.get("class", ""))
	if class_id != "":
		qualifiers.append(class_label(class_id).to_lower())
	if not qualifiers.is_empty():
		text += " (%s)" % ", ".join(qualifiers)
	return text


# --- Specs en sections (IB1, ADR 0109) -----------------------------------------------------
# Spec : `id`, `kind`, `headline_kind` (variante de `headline` du style), `title`, `subtitle`,
# `icon`, `icon_category`, `headline` [{key, icon, label, value}], `stats` [{key, label, value}],
# `effects` [{key, text, sign, before, after}] (`sign` : 1 favorable, -1 défavorable, 0 neutre),
# `traits` {strengths, weaknesses, abilities}, `requires` [{text, met}] (`met` : true, false ou
# null si l'état est inconnu), `warnings`, `flavour`, `footer` {cost, upkeep, time}, `detail`.
# Textes en BBCode (icônes, liens du Codex). `before`/`after` : fournis par le core dans
# `live["before_after"]` ({clé d'effet: [avant, après]}) — rien n'est calculé ici.


static func _spec(kind: String, id: String, title: String, subtitle: String, icon_category: String) -> Dictionary:
	return {
		"id": id, "kind": kind, "headline_kind": kind, "title": title, "subtitle": subtitle,
		"icon": id, "icon_category": icon_category, "headline": [], "stats": [], "effects": [],
		"traits": {"strengths": [], "weaknesses": [], "abilities": []}, "requires": [],
		"warnings": [], "flavour": "", "footer": {}, "detail": [],
	}


## Chiffres vedettes : ceux de `candidates` que `headline[variant]` du style retient, dans l'ordre.
static func _headline(variant: String, candidates: Dictionary) -> Array:
	var picked: Array = []
	for key in (TooltipView.style().get("headline", {}) as Dictionary).get(variant, []):
		if candidates.has(key):
			var item: Dictionary = (candidates[key] as Dictionary).duplicate()
			item["key"] = str(key)
			picked.append(item)
	return picked


## Ligne d'effet de spec (un effet par ligne) ; sens favorable selon `lower_is_better` du style.
static func effect_item(effect: Dictionary, live: Dictionary = {}) -> Dictionary:
	var kind: String = str(effect.get("kind", effect.get("effect", "")))
	var value := float(effect.get("value", 0))
	var sign := 0 if is_zero_approx(value) else (1 if value > 0 else -1)
	if kind in (TooltipView.style().get("lower_is_better", []) as Array):
		sign = -sign
	var item := {"key": kind, "text": effect_text(effect), "label": effect_label(kind), "value": effect_value(effect) + effect_qualifiers(effect), "sign": sign}
	# IB5 : un effet ciblé (classe sociale, famille d'unités) a sa propre clé « kind:cible ».
	var target: String = str(effect.get("class", "")) if str(effect.get("class", "")) != "" else str(effect.get("unit_category", ""))
	var lookup := kind if target == "" or target == "<null>" else "%s:%s" % [kind, target]
	var pair: Variant = (live.get("before_after", {}) as Dictionary).get(lookup, null)
	if pair is Array and (pair as Array).size() == 2:
		item["before"] = pair[0]
		item["after"] = pair[1]
		# Jauges à équilibre (santé, richesse…) : le core donne la valeur visée, pas l'immédiate.
		item["equilibrium"] = kind in (TooltipView.style().get("equilibrium_effects", []) as Array)
	return item


## Texte d'une ligne d'effet : « Moral 60 → 65 (+5) » si avant/après connus, sinon « Moral +5 » ;
## jauge à équilibre : « Santé : équilibre 50 → 55 (+5) ».
static func effect_line(item: Dictionary) -> String:
	if item.has("before") and item.has("after") and item.has("label"):
		var label := "%s : équilibre" % item["label"] if bool(item.get("equilibrium", false)) else str(item["label"])
		return "%s %s → %s (%s)" % [label, _number(float(item["before"])), _number(float(item["after"])), item.get("value", "")]
	return str(item.get("text", ""))


## Prérequis remplis : vrai si `live` dit l'action disponible, inconnu (null) sinon.
static func _met(live: Dictionary) -> Variant:
	return true if bool(live.get("available", false)) else null


## IB5 : état du prérequis `id` lu dans `live["requirements"]` ([{id, met}], calculé par le core),
## sinon `fallback`.
static func _requirement_met(live: Dictionary, id: String, fallback: Variant) -> Variant:
	for row in live.get("requirements", []):
		if row is Dictionary and str(row.get("id", "")) == id:
			return bool(row.get("met", false))
	return fallback


static func _icon_text(id: String, name: String) -> String:
	var icon := icon_bbcode(id, 14)
	name = entity_link(id, name)  # IB4 : bulle riche de l'entité
	return "%s %s" % [icon, name] if icon != "" else name


## Avertissements communs : importation de matériaux, indisponibilité, `live["warnings"]`.
static func _live_warnings(live: Dictionary) -> Array:
	var warnings: Array = []
	if int(live.get("import_cost", 0)) > 0:
		warnings.append("Dont importation : %s %s (%s manquant)" % [thousands(int(live["import_cost"])), POUND, cost_text({"resources": live.get("imported", {})})])
	if not live.is_empty() and not bool(live.get("available", true)):
		warnings.append("Indisponible : %s" % str(live.get("reason", "conditions non remplies")))
	for warning in live.get("warnings", []):
		warnings.append(str(warning))
	return warnings


## Lignes d'effets passées par `live["effects"]` ([{text, sign}], ex. état en bataille).
static func _live_effects(live: Dictionary) -> Array:
	var effects: Array = []
	for effect in live.get("effects", []):
		if effect is Dictionary and effect.has("text"):
			effects.append({"key": str(effect.get("key", "")), "text": str(effect["text"]), "sign": int(effect.get("sign", 0))})
	return effects


## BBCode d'une spec (repli, tests, Codex) : toutes les sections, version complète.
static func to_bbcode(spec: Dictionary) -> String:
	if spec.is_empty():
		return ""
	var lines: Array = [_title(str(spec.get("icon", "")), str(spec.get("title", "")), str(spec.get("subtitle", "")), str(spec.get("icon_category", "")), str(spec.get("id", "")))]
	var parts := PackedStringArray()
	for item in spec.get("headline", []):
		parts.append("%s : %s" % [item.get("label", ""), item.get("value", "")])
	lines.append(" · ".join(parts))
	var footer: Dictionary = spec.get("footer", {})
	parts = PackedStringArray()
	for key in ["cost", "upkeep", "time"]:
		if str(footer.get(key, "")) != "":
			parts.append("%s : %s" % [FOOTER_LABELS[key], footer[key]])
	lines.append(" · ".join(parts))
	for effect in spec.get("effects", []):
		var text := effect_line(effect)
		var sign := int(effect.get("sign", 0))
		lines.append("[color=%s]%s[/color]" % [GREEN if sign > 0 else RED, text] if sign != 0 else text)
	parts = PackedStringArray()
	for stat in spec.get("stats", []):
		parts.append("%s %s" % [stat.get("label", ""), stat.get("value", "")])
	lines.append(" · ".join(parts))
	var traits: Dictionary = spec.get("traits", {})
	if not (traits.get("strengths", []) as Array).is_empty():
		lines.append("[color=%s]Forces : %s[/color]" % [GREEN, ", ".join(PackedStringArray(traits["strengths"]))])
	if not (traits.get("weaknesses", []) as Array).is_empty():
		lines.append("[color=%s]Faiblesses : %s[/color]" % [RED, ", ".join(PackedStringArray(traits["weaknesses"]))])
	if not (traits.get("abilities", []) as Array).is_empty():
		lines.append("Capacités : " + ", ".join(PackedStringArray(traits["abilities"])))
	parts = PackedStringArray()
	for requirement in spec.get("requires", []):
		parts.append(str(requirement.get("text", "")))
	if not parts.is_empty():
		lines.append("%s : %s" % [str(spec.get("requires_label", "Requiert")), ", ".join(parts)])
	lines.append_array(spec.get("detail", []))
	lines.append(_flavour_bbcode(str(spec.get("flavour", ""))))
	for warning in spec.get("warnings", []):
		lines.append("[color=%s]%s[/color]" % [RED, warning])
	return _join(lines)


const FOOTER_LABELS := {"cost": "Coût", "upkeep": "Entretien", "time": "Durée"}


static func _flavour_bbcode(text: String) -> String:
	return "[color=%s][i]%s[/i][/color]" % [MUTED, text] if text != "" else ""


static func _effects_block(effects: Array, title: String = "Effets") -> String:
	var parts := PackedStringArray()
	for effect in effects:
		if effect is Dictionary:
			var kind := str(effect.get("kind", effect.get("effect", "")))
			parts.append(link_rule_label(effect_text(effect), kind, effect_label(kind)))
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
		parts.append("%s %s %d" % [icon_bbcode(str(res_id), 14), entity_link(str(res_id), GameCatalog.display_name(str(res_id))), int(resources[res_id])])
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


# --- Infobulles brutes (IB2, spec § 2.4) ---------------------------------------------------
# Les ~120 `tooltip_text = "…"` littéraux passent par une clé stable (« battle_log_toggle »…) du
# bloc `plain` de `data/ui/tooltips.json` (`title`, `body`, `hint`), au lieu du texte français en
# dur dans le script. `attach_plain` pose la clé sur le contrôle et s'assure qu'il rend la spec en
# sections (script générique `plain_tooltip_host.gd` si le contrôle n'est pas déjà `RichButton`,
# `IconChip` ou `RichPanel`). Textes dynamiques (`%`) : partie variable dans `live["body"]` (et
# éventuellement `live["title"]`/`live["hint"]`), qui l'emporte sur celle de `tooltips.json`.


## Entrée brute `key` du bloc `plain` : {title, body, hint}, valeurs vides si `key` est absente.
static func plain_entry(key: String) -> Dictionary:
	var entry: Variant = (texts().get("plain", {}) as Dictionary).get(key, null)
	return entry if entry is Dictionary else {}


## Spec `kind: "plain"` (titre + corps + raccourci en pied) de la clé `key` de `tooltips.json`,
## `live` (`title`/`body`/`hint`) prioritaire pour la part dynamique d'un texte.
static func plain_spec(key: String, live: Dictionary = {}) -> Dictionary:
	var entry := plain_entry(key)
	var title: String = str(live.get("title", entry.get("title", key)))
	var body: String = str(live.get("body", entry.get("body", "")))
	var hint: String = str(live.get("hint", entry.get("hint", "")))
	var spec := _spec("plain", key, title, "", "")
	spec["icon"] = ""
	# Ligne neutre (`sign: 0`) plutôt que `detail` : rendue en version courte comme en complète
	# (`TooltipView.blocks_for` ne montre `detail` qu'en version verrouillée).
	if body != "":
		spec["effects"].append({"key": "", "text": body, "sign": 0})
	if hint != "":
		spec["effects"].append({"key": "", "text": "[color=%s](%s)[/color]" % [MUTED, hint], "sign": 0})
	return spec


## BBCode d'une infobulle brute (repli, `to_bbcode`) : `title`/`body`/`hint` fournis directement
## (déjà résolus depuis `tooltips.json` par l'appelant, ex. `plain_spec`).
static func plain(title: String, body: String = "", hint: String = "") -> String:
	return to_bbcode(plain_spec("", {"title": title, "body": body, "hint": hint}))


## Attache une infobulle brute `ib:plain:<key>` à `control` : pose `tooltip_text` (clé + repli
## BBCode) et, si `control` n'est pas déjà une classe à infobulle riche (`RichButton`, `IconChip`,
## `RichPanel`…, reconnue à son script), lui attache le script générique `plain_tooltip_host.gd`
## qui route `_make_custom_tooltip` vers `panel_for`. `live` : `title`/`body`/`hint` dynamiques
## (ex. « Vitesse ×%d » selon la donnée du moment), sinon ceux de `tooltips.json`.
static func attach_plain(control: Control, key: String, live: Dictionary = {}) -> void:
	if control.get_script() == null:
		control.set_script(PLAIN_HOST_SCRIPT)
	elif not control.has_method("_make_custom_tooltip"):
		# Q8 : sans cette méthode, Godot affiche la clé et le BBCode bruts.
		push_error("attach_plain: %s (%s) lacks _make_custom_tooltip (raw tooltip)" % [control.name, control.get_script().resource_path])
	set_tooltip(control, "plain", key, live)


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
			strengths.append(stat_label(stat).to_lower())
		elif value <= average * 0.7 and stat != "siege_attack" and stat != "charge":
			weaknesses.append("ne tire pas" if stat == "ranged" and value <= 0.0 else stat_label(stat).to_lower())
	for ability in definition.get("abilities", []):
		if str(ability) == "rain_penalty":
			weaknesses.append(ability_label(ability))
	return [strengths, weaknesses]


## `live` : ligne de `get_recruitable` (`cost`, `upkeep`, `available`, `reason`, SV2 `resources`,
## `import_cost`, `imported`) ou unité
## d'armée (`strength`, `max_strength`, `morale`) ; vide pour la seule définition.
static func unit(unit_type: String, live: Dictionary = {}) -> String:
	return to_bbcode(unit_spec(unit_type, live))


## IB1 : spec en sections d'un type d'unité (même `live` que `unit`, plus `effects` [{text,
## sign}] et `warnings` pour l'état en bataille de `UnitCard`).
static func unit_spec(unit_type: String, live: Dictionary = {}) -> Dictionary:
	var definition := GameCatalog.unit_type(unit_type)
	var name: String = str(live.get("name", ""))
	if name == "":
		name = GameCatalog.display_name(unit_type)
	var category: String = str(definition.get("category", ""))
	var subtitle := unit_category_label(category)
	if definition.has("soldiers"):
		subtitle += ", %d hommes" % int(definition["soldiers"])
	var spec := _spec("unit", unit_type, name, subtitle, "unit")
	var stats: Dictionary = definition.get("stats", {})
	var candidates := {}
	var in_army := live.has("strength")
	if in_army:
		candidates["strength"] = {"icon": "gauge_strength", "label": "Effectif", "value": "%d / %d" % [int(live.get("strength", 0)), int(live.get("max_strength", 0))]}
		candidates["morale"] = {"icon": "gauge_morale", "label": "Moral", "value": str(int(live.get("morale", 0)))}
	var cost := ""
	if live.has("cost"):
		cost = "%s %s" % [thousands(int(live["cost"])), POUND]
	elif definition.has("cost"):
		cost = cost_text(definition["cost"])
	if cost != "":
		candidates["cost"] = {"icon": "hud_treasury", "label": "Coût", "value": cost}
	var main_stat := "ranged" if float(stats.get("ranged", 0)) > float(stats.get("melee", 0)) else "melee"
	if stats.has(main_stat):
		candidates["melee_or_ranged"] = {"icon": "stat_" + main_stat, "label": stat_label(main_stat), "value": _number(float(stats[main_stat])), "stat": main_stat}
	spec["headline_kind"] = "unit" if in_army else "unit_recruit"
	spec["headline"] = _headline(spec["headline_kind"], candidates)
	# Unité déjà levée : son prix de recrutement n'a plus d'intérêt au survol.
	var footer := {} if in_army else {"cost": cost}
	var upkeep := int(live.get("upkeep", definition.get("upkeep", -1)))
	if upkeep >= 0:
		footer["upkeep"] = "%s %s / saison" % [thousands(upkeep), POUND]
	if definition.has("recruit_time_turns"):
		footer["time"] = FrText.count(int(definition["recruit_time_turns"]), "tour")
	spec["footer"] = footer
	spec["effects"] = _live_effects(live)
	for stat in ["melee", "ranged", "range", "armor", "morale", "speed", "charge", "siege_attack", "ammo"]:
		if stats.has(stat) and (float(stats[stat]) > 0.0 or stat in ["melee", "armor", "morale"]):
			spec["stats"].append({"key": stat, "label": stat_label(stat), "value": _number(float(stats[stat]))})
	if not definition.is_empty():
		var sw := strengths_weaknesses(definition)
		spec["traits"]["strengths"] = Array(sw[0])
		spec["traits"]["weaknesses"] = Array(sw[1])
	for ability in definition.get("abilities", []):
		if str(ability) != "rain_penalty":
			spec["traits"]["abilities"].append(ability_label(ability))
	var met: Variant = _met(live)
	if str(definition.get("required_technology", "")) != "":
		var tech := str(definition["required_technology"])
		spec["requires"].append({"text": _icon_text(tech, GameCatalog.display_name(tech)), "met": _requirement_met(live, tech, met)})
	# B7c : le bâtiment requis se lit dans `enables_units` des bâtiments (seule source, lue par le core).
	var enablers := PackedStringArray()
	for building_id in enabling_buildings(str(definition.get("id", ""))):
		enablers.append(_icon_text(building_id, GameCatalog.display_name(building_id)))
	if not enablers.is_empty():
		spec["requires"].append({"text": " ou ".join(enablers) + " (ou supérieur)", "met": _requirement_met(live, "enabling_building", met)})
	# SV2 : matériaux des engins, tirés des provinces productrices à la commande ; le manque est
	# importé et compté dans le coût (même règle que les chantiers, ADR 0053).
	var materials: Dictionary = live.get("resources", (definition.get("cost", {}) as Dictionary).get("resources", {}))
	if live.has("cost") and not materials.is_empty():
		spec["detail"].append("Matériaux : " + cost_text({"resources": materials}))
	if int(live.get("import_cost", 0)) <= 0 and not materials.is_empty():
		spec["detail"].append("Matériaux tirés de vos provinces productrices, sinon importés et payés.")
	var period := unit_period(definition)
	if period != "":
		spec["detail"].append("Époque : " + period)
	if str(definition.get("source_class", "")) != "":
		spec["detail"].append("Recrutés parmi : %s" % class_label(str(definition["source_class"])).to_lower())
	spec["flavour"] = str(definition.get("description", ""))
	spec["warnings"] = _live_warnings(live)
	return spec


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
	return to_bbcode(building_spec(building_id, live))


## IB1 : spec en sections d'un bâtiment (même `live` que `building`).
static func building_spec(building_id: String, live: Dictionary = {}) -> Dictionary:
	var definition := GameCatalog.building(building_id)
	var name: String = str(live.get("name", ""))
	if name == "":
		name = GameCatalog.display_name(building_id)
	var category: String = str(definition.get("category", live.get("category", "")))
	var subtitle := building_category_label(category)
	if definition.has("tier"):
		subtitle += ", rang %d" % int(definition["tier"])
	var spec := _spec("building", building_id, name, subtitle, "building")
	spec["requires_label"] = "Prérequis"
	var effects: Array = []
	for effect in definition.get("effects", []):
		if effect is Dictionary:
			effects.append(effect_item(effect, live))
	var candidates := {}
	if not effects.is_empty():
		var main: Dictionary = effects[0]
		candidates["main_effect"] = {"icon": "", "label": main.get("label", ""), "value": main.get("value", ""), "sign": main.get("sign", 0)}
	spec["headline"] = _headline("building", candidates)
	# IB5 : l'effet principal garde sa ligne « avant → après » quand le core la fournit.
	if not (spec["headline"] as Array).is_empty() and str(spec["headline"][0].get("key", "")) == "main_effect" and not (effects[0] as Dictionary).has("before"):
		effects.remove_at(0)
	var footer := {}
	if live.has("cost"):
		footer["cost"] = "%s %s" % [thousands(int(live["cost"])), POUND]
	elif definition.has("cost"):
		footer["cost"] = cost_text(definition["cost"])
	var turns := int(live.get("turns", definition.get("build_time_turns", 0)))
	if turns > 0:
		footer["time"] = FrText.count(turns, "tour")
	footer["upkeep"] = "%s %s / saison" % [thousands(int(live.get("upkeep", definition.get("upkeep", 0)))), POUND]
	spec["footer"] = footer
	var units := PackedStringArray()
	for unit_id in definition.get("enables_units", []):
		units.append(_icon_text(str(unit_id), GameCatalog.display_name(str(unit_id))))
	if not units.is_empty():
		effects.append({"key": "enables_units", "text": "Permet de lever : " + ", ".join(units), "sign": 0})
	spec["effects"] = effects + _live_effects(live)
	var met: Variant = _met(live)
	for key in ["upgrades_from", "required_building", "required_technology", "required_resource"]:
		var value: String = str(definition.get(key, ""))
		if value != "":
			var note := ""
			if key == "upgrades_from":
				note = " (amélioration : le remplace et garde ses effets)"
			elif key == "required_building" and has_upgrade(value):
				note = " ou supérieur"
			spec["requires"].append({"text": _icon_text(value, GameCatalog.display_name(value)) + note, "met": _requirement_met(live, value, met)})
	if bool(definition.get("requires_coastal", false)):
		spec["requires"].append({"text": "province côtière", "met": _requirement_met(live, "coastal", met)})
	if bool(definition.get("requires_river", false)):
		spec["requires"].append({"text": "rivière", "met": _requirement_met(live, "river", met)})
	var has_materials := definition.has("cost") and (definition["cost"] as Dictionary).has("resources")
	if live.has("cost") and has_materials:
		spec["detail"].append("Matériaux : " + cost_text({"resources": definition["cost"]["resources"]}))
	# B7c : matériaux tirés des provinces productrices ; le manque est importé et compté dans le coût.
	if int(live.get("import_cost", 0)) <= 0 and has_materials:
		spec["detail"].append(RuleValues.format("Matériaux tirés de vos provinces productrices, sinon importés (prix de base × {rule.resource_import_multiplier})."))
	spec["flavour"] = str(definition.get("description", ""))
	spec["warnings"] = _live_warnings(live)
	return spec


# --- Technologies -------------------------------------------------------------------------


const TECH_STATE_LABELS := {"known": "Acquise", "researching": "En cours", "available": "Disponible", "locked": "Verrouillée"}


## `node` : entrée de `get_tech_tree` (effets, coûts effectif et de base, prérequis, date).
static func technology(node: Dictionary) -> String:
	return to_bbcode(technology_spec(node))


## IB1 : spec en sections d'une technologie (`node` : entrée de `get_tech_tree`).
static func technology_spec(node: Dictionary) -> Dictionary:
	var id: String = str(node.get("id", ""))
	var state: String = str(node.get("state", ""))
	var branch: String = str(node.get("branch", ""))
	var subtitle := "%s, rang %d" % [TECH_BRANCH_LABELS.get(branch, branch), int(node.get("tier", 1))]
	if state != "":
		subtitle += " — " + str(TECH_STATE_LABELS.get(state, state))
	var spec := _spec("technology", id, str(node.get("name", GameCatalog.display_name(id))), subtitle, "technology")
	spec["requires_label"] = "Prérequis"
	var cost := int(node.get("cost", 0))
	var effective := int(node.get("effective_cost", cost))
	var value := "%d" % effective
	if int(node.get("progress", 0)) > 0 and state != "known":
		value = "%d / %d" % [int(node.get("progress", 0)), effective]
	spec["headline"] = _headline("technology", {"research_cost": {"icon": "hud_research", "label": "Recherche", "value": value}})
	if (spec["headline"] as Array).is_empty():
		spec["footer"] = {"cost": "%d points" % effective}
	if effective > cost:
		spec["warnings"].append("En avance sur son temps : %d + %s %% de points" % [cost, RuleValues.text("anachronism_surcharge_percent")])
	for effect in node.get("effects", []):
		if effect is Dictionary:
			spec["effects"].append(effect_item(effect, node))
	var unlocks: Dictionary = node.get("unlocks", {})
	var unlocked := PackedStringArray()
	for unit_id in unlocks.get("units", []):
		unlocked.append(_icon_text(str(unit_id), GameCatalog.display_name(str(unit_id))))
	for building_id in unlocks.get("buildings", []):
		unlocked.append(_icon_text(str(building_id), GameCatalog.display_name(str(building_id))))
	if not unlocked.is_empty():
		spec["effects"].append({"key": "unlocks", "text": "Débloque : " + ", ".join(unlocked), "sign": 0})
	var met: Variant = null if state == "locked" or state == "" else true
	for prereq in node.get("prerequisites", []):
		spec["requires"].append({"text": _icon_text(str(prereq), GameCatalog.display_name(str(prereq))), "met": _requirement_met(node, str(prereq), met)})
	var year := int(node.get("historical_year", 0))
	if year > 0:
		spec["detail"].append("Date historique : %s%d" % ["vers " if bool(node.get("historical_uncertain", false)) else "", year])
	var herbs := herbs_line(node.get("herbs", []))
	if herbs != "":
		spec["detail"].append(herbs)
	var note: String = str(node.get("historical_note", ""))
	if note != "":
		spec["detail"].append(_flavour_bbcode(note))
	spec["flavour"] = str(node.get("description", ""))
	return spec


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
	lines.append(RuleValues.format(str(LENT_RULE_TEXTS.get(str(option.get("lent_rule", "none")), ""))))
	lines.append(RuleValues.format(str(WINTER_RULE_TEXTS.get(str(option.get("winter_rule", "none")), ""))))
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
	var lines: Array = [_title(resource_id, GameCatalog.display_name(resource_id), resource_category_label(category), "resource")]
	if stock >= 0:
		lines.append("Quantité accessible : %d" % stock)
	if definition.has("base_price"):
		lines.append("Prix de base : %s %s" % [_number(float(definition["base_price"])), POUND])
	var classes := PackedStringArray()
	for class_id in definition.get("satisfies_classes", []):
		classes.append("%s %s" % [icon_bbcode("class_" + str(class_id), 14), class_label(class_id).to_lower()])
	if not classes.is_empty():
		lines.append("Satisfait : " + ", ".join(classes))
	elif definition.has("satisfies_classes"):
		lines.append("Satisfait : aucune classe (matériau)")
	lines.append(_description(definition))
	return _join(lines)


static func population_class(class_id: String, data: Dictionary = {}) -> String:
	var lines: Array = [_title("class_" + class_id, class_label(str(class_id)), "", "class")]
	if data.has("count"):
		lines.append("Population : %s" % thousands(int(data["count"])))
	var gauges := PackedStringArray()
	for key in ["unrest", "health", "wealth", "goods_satisfaction"]:
		if data.has(key):
			gauges.append("%s %s %d" % [icon_bbcode("gauge_" + key, 14), rule_link(key, gauge_entry(key)[0]), int(data[key])])
	if not gauges.is_empty():
		lines.append(" · ".join(gauges))
	return _join(lines)


static func gauge(key: String, value: float = -1.0) -> String:
	var spec: Array = gauge_entry(key)
	var head := _title("gauge_" + key, str(spec[0]), "%d / 100" % int(round(value)) if value >= 0.0 else "", "gauge")
	return _join([head, RuleValues.format(str(spec[1]))])


static func hud(id: String, extra: String = "") -> String:
	var spec: Array = hud_entry(id)
	return _join([_title(id, str(spec[0]), "", "hud"), RuleValues.format(str(spec[1])), extra])


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
	# DA7c : icône propre au trait (`data/ui/icons_ink.json`, groupe "trait") ; repli sur
	# l'icône de catégorie générique si ce trait n'en a pas (`IconLibrary.resolve`).
	var lines: Array = ["%s [b]%s[/b]  [color=%s][i]%s[/i][/color]" % [icon_bbcode(id, 28, "trait"), name, MUTED, trait_category_label(category)]]
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
		branch_label(branch), int(node.get("tier", 1)), " — " + state if state != "" else ""]
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
	var head := _title("branch_" + branch_id, branch_label(branch_id), "niveau %d" % value if value >= 0 else "", "branch")
	return _join([head, branch_text(branch_id)])


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



# --- Liens et bulles filles (IB4, ADR 0109, spec IB § 3.3) ------------------------------------
# Les libellés d'effets, de stats et de jauges des infobulles en sections sortent comme liens
# `ib:rule:<clé>` (texte de la bulle : `data/ui/tooltips.json`, blocs `effects`, `stats`,
# `gauges`) ; les noms d'entités (prérequis, bâtiments habilitants, ressources, déblocages) comme
# liens `ib:<kind>:<id>` (bulle : spec riche complète). Libellés encore lus ici (IB2 migrera les
# tables) ; aucune règle de jeu.

const TEXTS_FILE := "ui/tooltips.json"
## Ordre de recherche d'une clé de règle dans `tooltips.json`.
const RULE_BLOCKS := ["effects", "stats", "gauges"]
const RULE_SUBTITLES := {"effects": "effet", "stats": "caractéristique d'unité", "gauges": "jauge"}
## Préfixe d'id → `kind` de lien d'entité.
const ENTITY_KINDS := {
	"unit_": "unit", "bld_": "building", "tech_": "technology", "res_": "resource",
	"trait_": "trait", "skill_": "skill",
}

const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

static var _texts: Dictionary = {}
## Vrai si les libellés viennent bien du fichier de données (tests).
static var texts_loaded_from_data: bool = false


## `data/ui/tooltips.json` (mis en cache). IB2 : repli comme `CameraFeel`/`TooltipView.style()` —
## le dossier de `MapPaths` peut ne pas avoir de `ui/tooltips.json` (fixtures de test), auquel cas
## on retombe sur `data/` à la racine du dépôt.
static func texts() -> Dictionary:
	if _texts.is_empty():
		var path := TooltipView._data_dir().path_join(TEXTS_FILE)
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(TEXTS_FILE)
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path)) if FileAccess.file_exists(path) else null
		if parsed is Dictionary:
			_texts = parsed
			texts_loaded_from_data = true
		else:
			push_warning("RichTooltip : %s illisible, bulles de règle sans texte." % TEXTS_FILE)
			_texts = {
				"effects": {}, "stats": {}, "gauges": {}, "hud": {}, "categories": {},
				"abilities": {}, "classes": {}, "branches": {}, "plain": {},
			}
			texts_loaded_from_data = false
	return _texts


## Relit `tooltips.json` au prochain accès (tests).
static func reload_texts() -> void:
	_texts = {}
	texts_loaded_from_data = false


## Entrée de règle `key` : {block, title, body (règles résolues), codex, icon} ; {} si aucune.
static func rule_entry(key: String) -> Dictionary:
	var data := texts()
	for block in RULE_BLOCKS:
		var entries: Dictionary = data.get(block, {})
		if entries.has(key) and entries[key] is Dictionary:
			var entry: Dictionary = entries[key]
			return {
				"block": block, "title": str(entry.get("title", entry.get("label", key))),
				"body": RuleValues.format(str(entry.get("body", ""))), "codex": str(entry.get("codex", "")),
				"icon": str(entry.get("icon", "")),
			}
	return {}


## Lien `ib:rule:<key>` sur `label` s'il existe un texte de règle, `label` seul sinon.
static func rule_link(key: String, label: String) -> String:
	return CodexText.ib_link("rule", key, label) if key != "" and not rule_entry(key).is_empty() else label


## `text` dont la première occurrence de `label` devient le lien de règle `key`.
static func link_rule_label(text: String, key: String, label: String) -> String:
	if label == "" or key == "" or text.contains("[url=%srule:%s]" % [CodexText.IB_PREFIX, key]):
		return text
	var at := text.find(label)
	if at < 0:
		return text
	var linked := rule_link(key, label)
	return text.substr(0, at) + linked + text.substr(at + label.length()) if linked != label else text


## Clé d'effet ou de stat dont le libellé est `label`, "" sinon.
static func rule_key_for_label(label: String) -> String:
	for block in ["effects", "stats"]:
		var entries: Dictionary = texts().get(block, {})
		for key in entries:
			if str((entries[key] as Dictionary).get("label", "")) == label:
				return str(key)
	return ""


## `kind` de lien d'une entité d'après le préfixe de son id, "" si inconnu.
static func entity_kind(id: String) -> String:
	for prefix in ENTITY_KINDS:
		if id.begins_with(prefix):
			return str(ENTITY_KINDS[prefix])
	return ""


## Nom d'entité en lien `ib:<kind>:<id>` (bulle riche), `label` seul si le type est inconnu.
static func entity_link(id: String, label: String) -> String:
	var kind := entity_kind(id)
	return CodexText.ib_link(kind, id, label) if kind != "" else label


## Spec d'une bulle ouverte par un lien `ib:<kind>:<id>` : règle (`kind` « rule »), spec riche de
## l'entité (`spec_for`), sinon spec simple tirée du BBCode de l'entité. {} si inconnue.
static func link_spec(key: String) -> Dictionary:
	var parts := key.split(":", true, 2)
	if parts.size() < 3 or parts[0] + ":" != KEY_PREFIX:
		return {}
	var kind: String = parts[1]
	var id: String = parts[2]
	if kind == "rule":
		return rule_spec(id)
	var spec := spec_for(key)
	if not spec.is_empty():
		return spec
	match kind:
		"resource":
			if not GameCatalog.resource(id).is_empty():
				return _bbcode_spec(kind, id, resource(id))
		"trait":
			if not GameCatalog.trait_definition(id).is_empty():
				return _bbcode_spec(kind, id, trait_tip({"id": id}))
		"skill":
			var definition := GameCatalog.skill(id)
			if not definition.is_empty():
				var node := definition.duplicate(true)
				node["name"] = GameCatalog.display_name(id)
				return _bbcode_spec(kind, id, skill(node))
	return {}


## Bulle de règle : titre, texte de `tooltips.json`, lien vers la fiche du Codex s'il y en a une.
static func rule_spec(key: String) -> Dictionary:
	var entry := rule_entry(key)
	if entry.is_empty():
		return {}
	var spec := _spec("rule", "", str(entry["title"]), str(RULE_SUBTITLES.get(entry["block"], "règle")), "gauge")
	var icon := str(entry.get("icon", ""))
	var library := icons()
	if icon == "" and library != null and bool(library.call("has_icon", "gauge_" + key)):
		icon = "gauge_" + key
	spec["icon"] = icon
	spec["rule_key"] = key
	if str(entry["body"]) != "":
		spec["detail"].append(str(entry["body"]))
	var codex := str(entry.get("codex", ""))
	if codex != "" and _codex_has(codex):
		spec["detail"].append("Codex : " + CodexText.link(codex))
		spec["codex"] = codex
	return spec


## Spec simple (en-tête + lignes) tirée du BBCode d'un constructeur pas encore en sections (IB2) :
## la première ligne (titre, sous-titre en italique) devient l'en-tête.
static func _bbcode_spec(kind: String, id: String, bbcode: String) -> Dictionary:
	var lines := bbcode.split("\n")
	var subtitle := ""
	var first := lines[0] if not lines.is_empty() else ""
	var sub_at := first.find("[i]")
	if sub_at >= 0:
		subtitle = first.substr(sub_at + 3, first.find("[/i]", sub_at) - sub_at - 3)
	var spec := _spec(kind, id, GameCatalog.display_name(id), subtitle, kind)
	for index in range(1, lines.size()):
		if lines[index] != "":
			spec["detail"].append(lines[index])
	return spec
