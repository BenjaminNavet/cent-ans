# JR3 — interface Godot de la faction croisée

Spec `docs/superpowers/specs/2026-10-02-jr-croises-jerusalem-design.md` § 4, 5, 5.1. Worktree
`../gp-jr` (`feat/jr`), partagé avec JR4 (qui possède `core/` et `data/rules/crusade.json`).
Périmètre : `game/` et cette note.

## État
- Section « Ferveur » (`game/scripts/ui/crusade_section.gd`) : jauge 0-100 teintée, traits aux
  seuils, losange au plancher, aumônes, état, bouton « Prêcher le passage », contingents attendus,
  infobulle des causes du tour. Posée sous le trésor dans `faction_panel.gd`.
- Repère du HUD : `MapUI.fervor_label` après le solde (`set_crusade`), clic → panneau de faction.
- Évènements `crusade` : `season_report.gd` (rubrique « Vos terres », style, délivrance publique),
  `end_turn_cluster.gd`, `news_letters.gd`, filtre du journal (`MapUI.journal_keeps`).
- Choix de faction : bannière d'une faction sans province dans `faction_map_picker.gd`
  (colonie tenue dans la province de la capitale) ; `_focus_capital` va sur l'ost.
- Test `game/tests/jr_crusade_test.gd` vert.

## Prochaine étape
- Script de capture `game/tests/jr_crusade_shot.gd`, smoke complet, relecture.

## Points ouverts
- Seuils (élan, moral, débandade) absents de `get_crusade()` : lus dans
  `data/rules/crusade.json` pour l'affichage ; à exposer par le pont.
- « Jérusalem délivrée » reconnue à son texte (un seul genre `crusade` dans le cœur).
- Clés d'infobulle : BBCode direct (`RichTooltip.plain`), pas d'entrée dans
  `data/ui/tooltips.json` (hors périmètre).
