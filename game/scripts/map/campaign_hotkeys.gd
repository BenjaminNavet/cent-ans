class_name CampaignHotkeys
extends RefCounted

## WH idle : raccourcis clavier de la carte de campagne qui servent à ne jamais oublier une armée
## ou une colonie. Les touches sont dans `project.godot` (InputMap), listées dans la fiche
## (`ShortcutSheet`) et réaffectables (`KeyBindings`) ; ici seulement la liste des actions et la
## logique pure du cycle. `MapUI._shortcut_input` émet `hotkey_pressed(action)`, `FlowController`
## l'exécute. Aucune règle de jeu.

## Actions routées vers `FlowController.handle_hotkey`.
const ACTIONS: Array[String] = [
	"campaign_next_idle", "campaign_prev_idle", "campaign_next_settlement", "campaign_capital",
	"campaign_end_turn_fast", "army_center", "army_follow", "campaign_commerce", "army_split", "army_garrison",
	"army_stance_normal", "army_stance_raid", "army_stance_siege", "army_stance_ambush",
	"army_stance_forced_march", "army_stance_entrenched",
]
const STANCE_PREFIX := "army_stance_"


## Posture (clé d'ordre `set_stance`) d'une action `army_stance_*`, ou "".
static func stance_of(action: String) -> String:
	return action.trim_prefix(STANCE_PREFIX) if action.begins_with(STANCE_PREFIX) else ""


## Élément suivant (`step` = 1) ou précédent (`step` = -1) de `ids` après `current`, cycliquement.
## `current` absent de la liste : le premier (suivant) ou le dernier (précédent). "" si vide.
static func next_in_cycle(ids: Array, current: String, step: int) -> String:
	if ids.is_empty():
		return ""
	var index := ids.find(current)
	if index < 0:
		return str(ids[0] if step >= 0 else ids[ids.size() - 1])
	return str(ids[posmod(index + (1 if step >= 0 else -1), ids.size())])
