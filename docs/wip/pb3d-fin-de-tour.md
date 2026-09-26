# PB3d — fin de tour dans un fil, rafraîchissement groupé

Branche `feat/pb3d-end-turn-thread` (worktree agent). Plan général : `docs/wip/pb3-performance.md`.
ADR : `docs/decisions/0081-fin-de-tour-dans-un-fil.md`.

## État
- [x] Pont Rust : `turn_job.rs` (fil de fin de tour, `resolve_turn` commun sync/async, test
  bit-à-bit), `campaign_sim_turn.rs` (`begin_end_turn`, `poll_end_turn`, `is_end_turn_pending`,
  `get_state_revision`), garde `refuse_while_turn_pending` sur toutes les méthodes qui modifient
  l'état (elle augmente aussi la révision).
- [x] Pont Rust : `campaign_sim_provinces.rs` (`get_provinces_snapshot(ids)` avec `constructing`,
  `get_settlements_live()` avec bâtiments/fortification/cité).
- [x] GDScript : `_on_end_turn(threaded)` (bouton/raccourci/confirmation → fil ; appels directs
  synchrones), `TurnWaitIndicator` « Les cours d'Europe délibèrent… », ordres refusés, sauvegarde
  manuelle refusée pendant le calcul.
- [x] GDScript : `ProvinceSnapshot` (cache par révision) dans couleurs politiques, frontières,
  parchemin, hameaux, vie des campagnes, alertes, intérêt des nouvelles, marqueurs de chantier ;
  croissance des colonies par `get_settlements_live`.
- [x] Bancs `pb1_turns.gd` et parcours RL1 : `total` jusqu'à la carte rafraîchie + pire image.
- [x] Smoke + tests de campagne headless (ct1, c5, m5a, fr1, mf1, m4, cv1, hud, settlements,
  ui3, ux2) verts avec la dylib debug de la branche.
- [ ] Mesures A/B (debug en cours, release à construire), fusion de `main`, cargo test complet.

## Conventions de mesure
- Dylibs de référence construites depuis 6a837e99 (base de la branche) dans le scratchpad ;
  script A/B : GDScript de 6a837e99 + dylib de base contre branche + nouvelle dylib, alternés.
- Cible partagée `core/target` : les worktrees partagent les noms d'artefacts (chemins
  relatifs) → un crate d'un autre worktree a été lié (erreur `vegetation::ReliefFloor`).
  Compilations de ce lot dans une cible privée du scratchpad.

## Mesures
(à venir)

## Prochaine étape
Mesures A/B debug puis release ; fusion de `main` ; rapport.
