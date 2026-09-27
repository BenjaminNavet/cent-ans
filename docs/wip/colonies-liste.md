# Liste des colonies (touche B) — suivi

Spec : `docs/superpowers/specs/2026-09-27-liste-colonies-design.md`.
Plan : `docs/superpowers/plans/2026-09-27-liste-colonies.md`.

## Lot L1 — TERMINÉ (2026-09-27)

`feat/holdings-core` : cœur (`sim-campaign::holdings::holdings_overview`, 7 tests
verts) et pont (`godot-bridge::campaign_sim_holdings::get_holdings_overview`)
livrés et vérifiés, fusionné dans `main` (8c926169).

Écart mineur : le champ `is_upgrade` de `SettlementRow` n'est pas dupliqué hors
de `options_available[].is_upgrade` — `upgrade_available` couvre le besoin de
l'IU.

## Lot L2 — Panneau Godot (`feat/holdings-ui`) — TERMINÉ (2026-09-27)

Touche retenue : **B** (`physical_keycode` 66, libre — vérifié dans
`project.godot` avant implémentation).

Livré :
- `game/scripts/map/holdings_controller.gd` (`HoldingsController`) : panneau en
  code façon « Mes unités », en-tête trésor/revenu net, filtres exclusifs
  (Tout / ⚒ libre / ▲ / ⚠), tri des provinces (revenu/nom/agitation),
  provinces repliables avec bouton « ⌖ », lignes de colonie avec chantier,
  garnison, badge de promotion et danger, infobulles (options de construction,
  raisons du danger en français).
- Branchements : `campaign_map.gd` (`holdings_ctl`, refresh dans
  `refresh_all`), `project.godot` (action `map_toggle_holdings`, touche B),
  `map_ui.gd` (bouton « Colonies », glyphe ⛫), `shortcut_sheet.gd` (entrée
  fiche des raccourcis), exclusion mutuelle avec `UnitRosterController`
  (une ligne dans chaque `toggle()`).
- `game/tests/holdings_test.gd` : ouverture par B, comptage des provinces vs
  aperçu, 3 filtres vs `count_idle`/`count_upgrade`/`count_endangered`, clic
  sur une colonie ouvre son panneau, exclusion mutuelle avec « Mes unités ».
  Vert.
- `game/tests/holdings_shot.gd` : capture de contrôle, écrit
  `docs/img/holdings.png` (non lue par cet agent, à juger par la session
  principale).
- `docs/manuel.md` : section « Liste des colonies (touche B) » (fin de la
  section 5).

Écart par rapport à la spec : l'ajout du bouton « Colonies » a fait déborder
la barre du haut à 1280×720 (`smoke.gd` : « top bar wider than the screen »).
Corrigé en ajoutant « Colonies » **et** « Unités » à `TOP_COLLAPSE_ORDER`
(`map_ui.gd`) — « Unités » n'y était pas encore, la barre était déjà à la
limite avant ce lot. Pas d'autre écart connu.

Vérifications finales : `holdings_test.gd`, `unit_roster_test.gd`, `smoke.gd`
verts (`core/build.sh` + `godot --headless --path game --import` faits au
préalable, cible cargo privée à ce worktree). Aucun changement Rust dans ce
lot (fmt/clippy/test déjà vérifiés par L1).

Fusionné dans `main` (bc53e337) après rebase ; tests Godot relancés après
rebase (holdings, unit_roster, smoke : verts). Capture jugée lisible par la
session principale. Worktrees supprimés. Aucune dépense cloud.

## Suites possibles
- Le conseil du tutoriel (en haut à gauche) chevauche le haut du panneau au
  premier tour.
- Playtest du joueur.
