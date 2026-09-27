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

## Lot L2 — Panneau Godot (`feat/holdings-ui`)

État : squelette commité (`HoldingsController` avec l'API publique en `pass`/
valeurs par défaut, `holdings_test.gd` désactivé). Touche retenue : **B**
(`physical_keycode` 66, libre — vérifié dans `project.godot`).

Branchements faits (campaign_map.gd, project.godot, map_ui.gd, shortcut_sheet.gd,
`shortcut_sheet.gd`, exclusion mutuelle avec `UnitRosterController`), puis le
panneau (§ 2 de la spec), puis les tests et la capture.
exclusion mutuelle avec `UnitRosterController`). Prochaine étape : le panneau
lui-même (§ 2 de la spec), puis les tests et la capture.

Panneau implémenté (`holdings_controller.gd` complet : en-tête trésor/revenu,
filtres exclusifs, tri, provinces repliables, lignes de colonie, infobulles).
`smoke.gd` vert après correction : bouton « Colonies » ajouté à
`TOP_COLLAPSE_ORDER` (glyphe ⛫), et « Unités » y a été ajouté aussi (déjà
limite avant, un bouton de plus suffisait à dépasser la largeur à 1280×720).
Prochaine étape : activer `holdings_test.gd`, puis la capture.
