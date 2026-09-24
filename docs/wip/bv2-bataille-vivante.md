# Lot BV2 — Bataille vivante (2) : morts, chutes, chocs de cavalerie, sang, démembrements

Branche `worktree-agent-af32527b47b35b47a` (a fusionné `main` puis le lot V2
`worktree-agent-aa60537d86d664cc6`). Backlog : `docs/audit/backlog-tw.md` § Bataille (idées J).
ADR : `docs/decisions/0022-chocs-et-morts-rendus.md` (à écrire).
Coordination : BV1 (flèches, sang au sol, taille des unités), V3 (feu, lumière), AU1 (audio :
`BattleAudio.play_at`, appelé dynamiquement s'il existe).

## État
| Lot | État |
|---|---|
| 0. Squelette : cœur `impact.rs` (événements d'impact, cause des pertes), pont, données `data/fx/battle_gore.json` + schéma | **fait** |
| 1. Morts et chutes (clip selon la cause, cavalier désarçonné, cheval abattu, cadavres par cellules avec LOD et plafond) | à faire |
| 2. Chocs de cavalerie (renversés, projetés, se relèvent ; chevaux ralentis ; piques et pieux) | cœur fait, rendu à faire |
| 3. Sang sur les figurines (taches, gerbes, chevaux, réglage « Sang ») | à faire |
| 4. Démembrements (réglage « complet ») | à faire |
| 5. Corrections V2 (pieds qui glissent, vouge à deux mains) | à faire |
| Mesures FPS `--units=50`, captures `docs/audit/captures/bv2/` | à faire |

## Cœur (fait)
- `core/crates/sim-battle/src/impact.rs` : `ImpactEvent` (attaquant, défenseur, sorte
  shock/pikes/stakes/broken, point de contact, cap, poids de charge, renversés, désarçonnés,
  profondeur, cohésion), `LossCause` (arrow, bolt, ball, stone, melee, charge, stakes, pikes,
  fire, other) ; fonctions de règle déterministes (`charge_mass`, `knocked_count`,
  `pikes_stop`, `drive_depth`, `shock_morale`).
- Règles ajoutées : les renversés ne combattent pas pendant `KNOCKDOWN_TIME` (3 s,
  `Unit::fighting_soldiers`) ; les piques (capacité schiltron, de front ou en carré) arrêtent la
  charge (8 % de cavaliers perdus, moral −10, journal). Cohésion = choc existant (8 / 15).
- `BattleSim::take_impacts()` (file bornée à 256) ; `Unit::loss_cause` / `loss_by`.
- Pont : `BattleSim.get_impacts()`, clés `loss_cause`, `loss_by`, `knocked` de `get_units()`.
- Tests `core/crates/sim-battle/tests/bv2.rs` ; empreinte `b6.rs` seed 3 : 49 → 50 pertes
  (renversés hors combat quelques secondes).

## Prochaine étape
Rendu : `battle_gore.gd` (gerbes, morceaux), cadavres par cause dans `battle_soldiers.gd`,
shader skinné (impulsion, masquage par os, sang codé dans `INSTANCE_CUSTOM.w`).
