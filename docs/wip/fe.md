# FE — féodalité et petites factions (orchestration)

Spec : `docs/superpowers/specs/2026-09-28-feodalite-design.md`. Plan : `docs/superpowers/plans/2026-09-28-feodalite.md`.
ADR : 0098. Worktree orchestrateur : `../game_project-fe` sur `feat/fe`.

## État (pause du 2026-09-28, à la demande du joueur)
- F0 dans `main` (1dcd6821) : titres, migration (45 titres), `feudal.rs` avec déductions, sauvegarde v7.
- Vague 1 terminée par les agents, branches : F1 `feat/fe1-deductions` (2fb75fde), F2 `feat/fe2-escalade`
  (5f2abba4), F3 `feat/fe3-titres` (a86b9152), F4a `feat/fe4a-france` (cddb3c5d). Rapports dans leurs `docs/wip/fe<N>-*.md`.
- Intégration **en cours** dans `../game_project-fe` (`feat/fe`, pas encore dans `main`) :
  - fusionnés et commités : F3, F2 (0433b2b2), F4a (52d3a711), schéma d'objectifs resserré, budget à 6 colonnes,
    8 tests rouges après F4a corrigés (591cef00 ; détail dans `docs/wip/fe-integration-v1.md`).
  - **fusion de F1 interrompue** : conflits non résolus dans `core/crates/sim-campaign/src/feudal.rs` et
    `characters.rs` (le reste de F1 est indexé). Reprendre là (ou `git merge --abort` puis refaire).

## Reprise
1. Finir la fusion F1 et réconcilier (voir `docs/wip/fe-integration-v1.md` et le brief ci-dessous) :
   - une seule convocation d'ost (F1 `rally_vassals` + F2 `summon_host`), refus → `feudal::on_host_refused` (F3) ;
   - `sync_suzerains` après chaque transfert/concession/vacance/destruction (F3) ; `liege_overrides` (F1)
     compatibles avec « titre supérieur vacant → indépendant » (F3) ;
   - mémoire de loyauté F1 alimentée par F2 (protection, défaites) et F3 (concession, commise, prétendant) ;
   - tests F2 : retirer l'alignement manuel de `suzerain`.
2. pytest : 16 échecs dus aux données F4a (colonies, horizon, navgrid, héraldique, front-end, portraits), à corriger.
3. Vérifications complètes (fmt, clippy, cargo test, pytest, build, smoke), ff `main`, puis vague 2 (F4b-F4e, F5).

## Points ouverts (à trancher en F5/F8)
- `ai/tests/g4.rs` (Brabant allié de l'Angleterre) en `#[ignore]` : un vassal peut-il s'allier hors de son suzerain ? (F5)
- Test 50 tours `m3_grid_ai` (`--ignored`) : rouge déjà sur main ; plus tôt avec les princes d'Empire vassaux (F8).
- Interprétations F3 à valider : conquête (§ 4.6), « plusieurs prétendants », lois de succession par titre (données à écrire).
- F6 : offres Protection/Arbitrage, ordre `ArbitratePrivateWar`, article `demand_title`, factions créées sans données d'affichage.
- F4a : Normandie en deux titres, héritiers manquants, couleurs héraldiques en doublon.
- Worktrees d'agents FE (F1-F4a) à supprimer après fusion dans `main`.
