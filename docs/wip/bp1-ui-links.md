# WIP — BP1 « bulles partout — couverture UI »

Branche : `bp1-ui-links` (worktree agent-a037a772189f79242).

## État
Tous les fichiers listés dans la tâche ont été branchés/formatés (point 1) :
- Journal de campagne (`game/scripts/map/map_ui.gd`) : `log_text` branché (`CodexBubbles.attach`
  dans `_ready`), texte de chaque événement passé par `CodexText.format(text, true)` avant
  l'habillage couleur/icône (`add_events`).
- Encyclopédie (`game/scripts/ui/encyclopedia.gd`) : `fiche` branché ; `_description()` (corps de
  toutes les fiches types/bâtiments/techs/factions/etc.) et le texte des fiches « Mécaniques »
  passés par `CodexText.format(…, true)`. Ordre : `attach` avant `meta_underlined = true` (sinon
  `attach` remet `meta_underlined` à false et casse le style des liens internes existants).
- Aide F1 (`game/scripts/map/help_controller.gd`) : `text` branché ; `HELP_TEXT` (pas la fiche des
  raccourcis `ShortcutSheet.bbcode()`) passé par `CodexText.format(…, true)`.
- Tutoriel (`game/scripts/ui/tutorial.gd`) : `text_label` (texte de l'étape + conseil) branché ;
  `mouse_filter` passé à `PASS` (était `IGNORE` via `_rich()`, ce qui aurait empêché tout survol) ;
  `tutorial_steps.gd` est un pur conteneur de données consommé par `tutorial.gd`, rien à y faire.
- Diplomatie (`game/scripts/ui/diplomacy_panel.gd`) : `_reasons` (motifs d'acceptation/refus d'un
  traité) et `_unilateral_hint` (conséquences d'une action unilatérale) branchés ; le texte
  `blocked` et chaque `reason["text"]` passés par `CodexText.format(…, true)`.
- HUD de bataille (`game/scripts/battle/battle_hud.gd`) : panneau d'aide F1 (`_build_help`)
  branché, `HELP_TEXT.format(...)` passé par `CodexText.format(…, true)`.
- Barre d'ordres (`game/scripts/battle/leader_orders_bar.gd`) : l'infobulle personnalisée
  (`OrderButton._make_custom_tooltip`) appelait déjà `CodexText.format` mais construisait son
  propre panneau à la main, sans jamais l'enregistrer auprès de `RichTooltip` — la touche T ne
  pouvait donc jamais la convertir en bulle épinglée. Remplacé par `RichTooltip.make_panel(bbcode)`
  qui applique le format, le style et l'enregistrement (`last_panel`/`last_bbcode`) communs à
  toutes les infobulles riches du jeu.
- Choix de faction (`game/scripts/ui/faction_select.gd`) : `_detail_intro` avait `bbcode_enabled =
  false` (texte brut) ; passé à `true`, branché, et le texte de description formaté.

## Point 2 — comptage 21/27 unités de l'encyclopédie
Cause non trouvée par lecture de code : `data/unit_types/` contient bien 27 fichiers, 27 ids
uniques (vérifié en Python), aucun filtre dans `Encyclopedia.build_entries()` ni `_fill_list()`
qui expliquerait une réduction à 21. Les 14 unités ajoutées par UR1 (commit 510b8aef) n'ont pas de
champ d'année/faction qui les exclurait de la liste (`required_faction` présent sur 5 d'entre
elles seulement, non utilisé comme filtre). Reste à reproduire en conditions réelles (le
`core/build.sh` de ce worktree partait de zéro — dépendances godot-rust complètes — et a pris
plus de 15 minutes). Prochaine étape : une fois la dylib prête, lancer
`CENT_ANS_SMOKE_ONLY=` (smoke complet, le test tutoriel n'a pas de filtre dédié) ou ajouter des
`print()` temporaires dans `build_entries()`/`_fill_list()` pour voir quels ids manquent à l'appel.

## Point 3 — smoke
Ajouté dans `game/tests/smoke.gd` (`_run_tutorial`, juste avant `map.queue_free()`) :
- `map.ui.log_text.has_meta("codex_attached")` (branchement du journal) ;
- un événement de test contenant l'alias « Crécy » ajouté via `map.ui.add_events(...)`, puis
  vérification que `log_text.text` contient `[url=cdx:` ;
- `map.help.text.has_meta("codex_attached")` et, après `map.help.toggle()`, que le texte de
  l'aide F1 contient `[url=cdx:` (alias connus : Crécy, la Peste noire, Jeanne d'Arc… déjà cités
  dans `HELP_TEXT`).

## Prochaine étape
1. Construire la dylib (`core/build.sh`, en cours dans ce worktree), `godot --headless --path
   game --import`, puis lancer le smoke complet et confirmer les nouveaux contrôles.
2. Élucider le 21/27 (voir ci-dessus) et corriger.
3. `game/tests/hover_probe.gd` en fenêtré sur un label du journal, une fois le smoke vert.
