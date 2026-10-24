# Onglet « Chantiers » du panneau de faction (client Bevy)

Spec : `docs/design/chantiers-royaume.md`. Noyau : `core/crates/sim-campaign/src/realm_works.rs` (fait).

## État
- `faction/works.rs` : logique pure + 4 tests. Onglet câblé (TABS[4], view.rs `works_tab`, `works_press` dans mod.rs), dump `panel_faction.works`, README, scénario smoke `client/tests/smoke/faction_works.txt`.
- clippy + `cargo test --features dev` verts.

## Prochaine étape
Exécuter le scénario smoke `faction_works` et lire `out/smoke/works_*.json`; puis smoke complet.

## Points ouverts
- Le noyau ne s'arrête pas à l'épuisement des fonds : une entrée refusée n'empêche pas une entrée moins chère suivante (comportement de `apply_works_batch`) ; le récapitulatif l'annonce.
