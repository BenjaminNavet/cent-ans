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
- Terminé : `feudal_titles.rs` 11 tests verts ; `cargo test --workspace` (148 binaires ok),
  clippy `-D warnings` propre ; pytest schémas/titres verts.

## Choix d'interprétation
- § 4.6 lu ainsi : le vainqueur usurpe un titre de rang **supérieur ou égal** à son titre
  principal ; un titre inférieur est concédé à son vassal direct le plus puissant (gardé sans vassal).
- « Plusieurs prétendants » = héritier désigné ≠ héritier selon la loi, sous un suzerain ; le
  perdant rejoint l'ennemi le plus puissant du suzerain, qui reçoit une prétention au trône et
  déclare la guerre (guerre de succession).
- Extinction : héritier par titre parmi la parenté dans d'autres factions (maison, mère ou père de
  la maison), loi du titre sinon de la faction ; sans parent : déshérence au suzerain ; titre
  souverain sans parent : reste à la faction (régence/nouvelle maison comme avant).
- Titre vacant (faction détruite sans suzerain pour ses titres) : ses vassaux deviennent indépendants.

## Points ouverts
- F1 : appeler `feudal::on_host_refused` dans `rally_vassals` ; utiliser `forfeitures`/
  `disputes` pour les termes de loyauté `peer_forfeiture` et `rival_claimant`.
- F5 : arbitrage et commise sont automatiques/données, pas encore de décision IA/joueur.
- F6 : factions créées dynamiquement (`fac_<titre>`) sans données d'affichage ; article
  `demand_title` à exposer dans le panneau de diplomatie.
- Une faction joueur éteinte avec parent à l'étranger est absorbée (défaite) : à juger en F8.
