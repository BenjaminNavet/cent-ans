# FE — féodalité et petites factions (orchestration)

Spec : `docs/superpowers/specs/2026-09-28-feodalite-design.md`. Plan : `docs/superpowers/plans/2026-09-28-feodalite.md`.
ADR : 0098. Worktree orchestrateur : `../game_project-fe` sur `feat/fe`.

## État (2026-09-28, reprise)
- F0 dans `main` (1dcd6821). Vague 1 entièrement fusionnée dans `feat/fe` (`../game_project-fe`) :
  F3, F2, F4a, puis F1 (cdad6965) avec réconciliation :
  - une seule convocation d'ost : `feudal::summon_host` (l'attaquant et le suzerain protecteur convoquent
    leurs vassaux directs) ; refus → `on_host_refused` envers le suzerain qui convoque ;
  - `sync_suzerains` appelé à chaque changement de détention (`transfer::refresh_primary`) ; la déshérence
    suit le suzerain effectif (`effective_liege`, donc les hommages) ; titre supérieur vacant → indépendant ;
  - mémoire de loyauté F1 alimentée par concession (F3), commise (pairs du félon), succession contestée
    (prétendant rival) et paix perdue (côté qui cède terres ou tribut). La protection reste un modificateur
    d'opinion F2 (visible dans l'UI) : les variantes `Protection*` de F1 ont été retirées (double compte) ;
  - tests F2 : `suzerain` dérivé par `sync_suzerains`, plus d'alignement manuel.
- `main` (TW2, RS) fusionné dans `feat/fe` (3d634500) : fmt, clippy, 152 binaires de test verts, smoke OK.
- En cours : agent `feat/fe4a-assets` (`../game_project-fe4a-assets`) pour les 16 échecs pytest dus aux
  données F4a (colonies, horizon, navgrid, héraldique, front-end, portraits).

## Reprise
1. Fusionner `feat/fe4a-assets` dans `feat/fe`, pytest vert, ff `main`.
2. Vague 2 (F4b-F4e, F5) depuis `main`. Les artefacts globaux régénérés (navgrid, horizon) se régénèrent
   à l'intégration, pas dans chaque lot.

## Points ouverts (à trancher en F5/F8)
- `ai/tests/g4.rs` (Brabant allié de l'Angleterre) en `#[ignore]` : un vassal peut-il s'allier hors de son suzerain ? (F5)
- Test 50 tours `m3_grid_ai` (`--ignored`) : rouge déjà sur main ; plus tôt avec les princes d'Empire vassaux (F8).
- Interprétations F3 à valider : conquête (§ 4.6), « plusieurs prétendants », lois de succession par titre (données à écrire).
- F6 : offres Protection/Arbitrage, ordre `ArbitratePrivateWar`, article `demand_title`, factions créées sans données d'affichage.
- F4a : Normandie en deux titres, héritiers manquants, couleurs héraldiques en doublon.
- Worktrees d'agents FE (F1-F4a) à supprimer après fusion dans `main`.
