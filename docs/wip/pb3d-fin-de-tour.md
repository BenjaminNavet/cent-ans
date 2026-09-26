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
- [x] Mesures A/B debug et release (ci-dessous).
- [ ] Fusion de `main`, cargo test complet, smoke après fusion.

## Conventions de mesure
- Dylibs de référence construites depuis 6a837e99 (base de la branche) dans le scratchpad ;
  script A/B : GDScript de 6a837e99 + dylib de base contre branche + nouvelle dylib, alternés.
- Cible partagée `core/target` : les worktrees partagent les noms d'artefacts (chemins
  relatifs) → un crate d'un autre worktree a été lié (erreur `vegetation::ReliefFloor`).
  Compilations de ce lot dans une cible privée du scratchpad.

## Mesures (26/09, M4 Pro chargée, A/B alternés, médianes de 3 exécutions)
Base = GDScript + dylib de 6a837e99 ; nouveau = branche. `pb1_turns.gd` : médiane des tours 1-5
de chaque exécution ; parcours `-- --journey --uncapped --no-battle --frames=30 --turns=3`
(caméra sur Paris à 150, images déjà ≈ 50 ms). « total » = appel → carte rafraîchie ;
« pire image » = plus long intervalle entre deux images pendant ce temps.

| dylib | mesure | base | nouveau |
|---|---|---|---|
| debug | pb1 total | 215 ms | 210 ms |
| debug | pb1 pire image | 253 ms | 85 ms |
| debug | parcours total | 241 ms | 246 ms |
| debug | parcours pire image | 330 ms | 165 ms |
| release | pb1 total | 196 ms | 188 ms |
| release | pb1 pire image | 229 ms | 78 ms |
| release | parcours total | 220 ms | 239 ms |
| release | parcours pire image | 301 ms | 153 ms |

Détail nouveau (release, pb1) : lancement du fil (clone de l'état) 0,6-0,8 ms ; attente du fil
≈ 130 ms (cœur seul ≈ 114 ms) ; `refresh_all` ≈ 59 ms (base ≈ 80-95 ms estimé : total − cœur).
Sans qualité de service (première série debug) le fil tombait sur les cœurs d'efficacité :
attente 600-1200 ms dans une exécution sur trois → `pthread_set_qos_class_self_np(USER_INITIATED)`.
Reste de `refresh_all` (instrumentation temporaire) : marqueurs d'armée 8-50 ms (figurines
recréées pour les armées qui ont changé), vie des campagnes 4-45 ms (maquettes de croissance),
panneau diplomatique ouvert 18 ms, barre du haut 4 ms, routes commerciales 4 ms.

## Conflits attendus avec `fix/code-review` (pas encore dans `main` au 26/09 matin)
Essai de fusion : deux fichiers en conflit, résolution simple.
- `campaign_map.gd` `_on_end_turn` : garder ma version (`threaded`, `_resolve_end_turn`,
  `end_turns_refreshed`) et y ajouter le garde de la revue après `await ai_replay.play()`
  (`sim_before` → `return` si une autre partie a été chargée) ; `_on_save` : garder
  `_refuse_during_end_turn()` + `last_save_ok`/`SaveSlots.save` de la revue ; `_on_load` : garder
  le refus pendant le rejeu de la revue.
- `settlement_layer.gd` `refresh` : garder ma lecture `ProvinceSnapshot` et la mise à jour des
  seules tuiles de hameaux changées de la revue.
- Pont : la revue n'ajoute aucune méthode mutante ; toute future méthode `&mut self` de
  `CampaignSim` doit commencer par `refuse_while_turn_pending` (ADR 0081).

## Prochaine étape
Fusion de `main` (conflits attendus avec la revue de code : pont, campaign_map.gd) ; cargo
test/clippy/fmt ; smoke et tests de campagne ; rapport.
