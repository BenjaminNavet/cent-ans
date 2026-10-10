class_name DiplomacyCourtBanner
extends DiplomacyView

## Bandeau « Affaires de la Cour » en tête de l'écran de diplomatie : offres en attente,
## pactes et trêves qui expirent sous peu, ligue des princes active. Masqué s'il n'y a rien.
## Lecture seule de `get_offers`, `get_diplomacy` (`truce_turns_left`, `non_aggression_turns_left`)
## et `get_league`.

## Un pacte ou une trêve est « proche de son terme » sous ce nombre de tours.
const EXPIRY_HORIZON := 2


func _init() -> void:
	super("CourtAffairs", 4, false)
	hide()


## `entries` : fiches de `get_diplomacy` (le panneau les a déjà).
func show_for(simulation: Object, player: String, selected: String, faction_entry: Dictionary = {}, entries: Array = []) -> void:
	sim = simulation
	player_faction = player
	_entries_cache = entries
	_render()


var _entries_cache: Array = []


## Lignes du bandeau, chacune « glyphe + mots » (la couleur n'est jamais seule).
func lines() -> PackedStringArray:
	var out := PackedStringArray()
	if sim == null:
		return out
	var offers: Array = sim.call("get_offers")
	if not offers.is_empty():
		out.append("✉ %s à trancher" % FrText.count(offers.size(), "proposition", "propositions"))
	for e in _entries_cache:
		for pair in [["truce_turns_left", "Trêve"], ["non_aggression_turns_left", "Pacte de non-agression"]]:
			var turns := int(e.get(pair[0], 0))
			if turns > 0 and turns <= EXPIRY_HORIZON:
				out.append("⌛ %s avec %s : encore %s" % [pair[1], str(e["name"]), FrText.count(turns, "tour", "tours")])
	if sim.has_method("get_league"):
		var league: Dictionary = sim.call("get_league")
		if bool(league.get("active", false)):
			out.append("⚔ Ligue des princes contre %s%s" % [str(league.get("target_name", "?")), " (nous sommes liés à lui)" if bool(league.get("bound", false)) else ""])
	return out


func _render() -> void:
	UiBuild.clear_children(self)
	var rows := lines()
	visible = not rows.is_empty()
	if rows.is_empty():
		return
	add_child(_section("Affaires de la Cour"))
	for text in rows:
		var label := _label(text, UiType.BODY, HudStyle.INK)
		label.name = "CourtLine"
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		add_child(label)
