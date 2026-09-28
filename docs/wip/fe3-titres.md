# FE3 — transferts de titres (lot F3 du chantier FE)

Branche `feat/fe3-titres` (depuis `main` 1dcd6821). Plan : `docs/superpowers/plans/2026-09-28-feodalite.md` § F3.

## Découpage
- `sim-campaign/src/feudal.rs` : `FeudalState` étendu (`forfeitures`, `disputes`, `start_crowns`,
  `streaks`, `objectives_met`), stubs F0 remplis par délégation, `liege_of` (titre vacant = indépendance).
- `feudal/transfer.rs` : `transfer` (provinces propres du titre suivent, titre principal recalculé,
  faction sans titre absorbée = union personnelle), `vacate_title`, `on_faction_destroyed`
  (déshérence ou vacance), `grant_title` (courtisan → nouvelle faction vassale), `conquer_title`.
- `feudal/felony.rs` : `open_felony_towards`, `on_host_refused` (pour F1), `on_revolt`,
  `declare_commise`, `settle_forfeitures` (à la paix), détection d'alliance avec l'ennemi.
- `feudal/inherit.rs` : `contested_succession` (arbitrage du suzerain, perdant chez l'ennemi du
  suzerain qui déclare la guerre), `inherit_titles_on_extinction` (héritier par titre, déshérence).
- `feudal/objectives.rs` : objectifs du titre principal, victoire générique, phase `resolve_feudal`.
- Branchements (une ligne chacun) : `characters::succeed`, `resolve_faction_deaths`
  (`dissolve_faction` extrait), `diplomacy::make_peace_between`, `casus_belli`, révolte de vassal,
  `turn.rs`, `victory::resolve_victory`, `negotiation::Article::DemandTitle`.
- Données : `feudal.json` (`forfeiture_win_war_score`, `title_loss_penalty`, `arbitration`),
  schéma ; `succession_law` optionnelle par titre (`title.schema.json`, `FeudalTitle`).

## État
- Code complet, `feudal_titles.rs` : 11 tests verts. Suite complète + clippy en cours.

## Prochaine étape
- Vérifier la suite complète (régressions d'équilibre possibles : Bretagne 1341 se déclenche
  désormais en partie normale), commit final `FE3`.
