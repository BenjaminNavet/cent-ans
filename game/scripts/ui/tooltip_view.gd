class_name TooltipView
extends RefCounted

## Chantier IB (ADR 0109) : rendu en sections d'une spec d'infobulle produite par `RichTooltip`.
## Spec (dictionnaire, présentation pure) : `id`, `kind`, `title`, `subtitle`, `icon`,
## `headline` [{icon, label, value}], `stats` [{key, value}], `effects` [{text, sign, before,
## after}], `traits` {strengths, weaknesses, abilities}, `requires` [{text, met}], `warnings`,
## `flavour`, `footer` {cost, upkeep, time}, `detail` (lignes de la version complète).
## Blocs de haut en bas : en-tête, vedettes, effets, stats, forces/faiblesses, conditions,
## ambiance, pied ; filet entre blocs non vides. `detailed` : version complète (bulle
## verrouillée) ; sinon version courte (survol, `short_max_body_lines` lignes au plus).
## Style : `data/ui/tooltip_style.json`. Aucune règle de jeu ici.

## Lu comme `CameraFeel` : `MapPaths` donne le dossier `data/`.
const DATA_FILE := "ui/tooltip_style.json"


## Contrôle d'infobulle pour `spec`. Squelette IB0 : repli sur le panneau BBCode actuel (IB1).
static func build(spec: Dictionary, detailed: bool = false) -> Control:
	return RichTooltip.make_panel(str(spec.get("bbcode", spec.get("title", ""))))


## Style chargé depuis `data/ui/tooltip_style.json` (mis en cache). Squelette IB0 (IB1).
static func style() -> Dictionary:
	return {}
