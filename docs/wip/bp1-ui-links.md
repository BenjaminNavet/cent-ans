# WIP — BP1 « bulles partout — couverture UI »

Branche : `bp1-ui-links` (worktree agent-a037a772189f79242).

## État
- Journal de campagne (`game/scripts/map/map_ui.gd`) : `log_text` branché (`CodexBubbles.attach`),
  texte de chaque événement passé par `CodexText.format(text, true)` avant l'habillage couleur/icône.
- Encyclopédie (`game/scripts/ui/encyclopedia.gd`) : `fiche` branché ; `_description()` (corps de
  toutes les fiches types/bâtiments/techs/factions/etc.) et le texte des fiches « Mécaniques »
  passés par `CodexText.format(…, true)`. Ordre : `attach` avant `meta_underlined = true` (sinon
  `attach` remet `meta_underlined` à false et casse le style des liens internes existants).
- Aide F1 (`game/scripts/map/help_controller.gd`) : `text` branché ; `HELP_TEXT` (pas la fiche des
  raccourcis `ShortcutSheet.bbcode()`) passé par `CodexText.format(…, true)`.
- Reste à faire : tutoriel (`tutorial.gd` + `tutorial_steps.gd`), diplomatie (`diplomacy_panel.gd`),
  HUD de bataille (`battle_hud.gd`), barre d'ordres (`leader_orders_bar.gd`), choix de faction
  (`faction_select.gd`).
- Point 2 (comptage 21/27 unités) : pas encore élucidé — build en cours pour lancer le smoke ciblé
  et investiguer en conditions réelles (fixtures vs vraies données, cache `GameCatalog`).
- Point 3 (ajout au smoke) : pas commencé.

## Prochaine étape
Continuer le balayage des fichiers listés, puis lancer `godot --headless --path game --import`
et le smoke complet une fois la dylib construite.
