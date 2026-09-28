# FE3 — transferts de titres (lot F3 du chantier FE)

Branche `feat/fe3-titres` (depuis `main` 1dcd6821). Plan : `docs/superpowers/plans/2026-09-28-feodalite.md` § F3.

## Découpage
- `sim-campaign/src/feudal.rs` : état (`FeudalState` étendu), stubs délégués, `liege_of` (titre vacant = indépendance).
- `sim-campaign/src/feudal/transfer.rs` : transfert, fusion, création/disparition de faction, vacance, conquête, concession.
- `sim-campaign/src/feudal/felony.rs` : félonie, commise, exécution à la paix.
- `sim-campaign/src/feudal/inherit.rs` : succession contestée (arbitrage), extinction (héritiers par titre, déshérence).
- `sim-campaign/src/feudal/objectives.rs` : objectifs, séries de victoire générique, phase `resolve_feudal`.

## État
- Exploration faite ; implémentation en cours.

## Prochaine étape
- Coder les sous-modules, brancher `succeed`, `resolve_faction_deaths`, `make_peace_between`, `casus_belli`, `resolve_victory`, `turn.rs`, `Article::DemandTitle`.
