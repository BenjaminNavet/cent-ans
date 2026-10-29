# Lot R1 — renommée accumulée, actes à dates fixes, ambitions (audit 2026-10-29-campagne, R1-0/1/2)

Branche : `worktree-agent-adc1d3f985235a4e2` (worktree agent). Source : `docs/audits/2026-10-29-campagne.md` § 4 R1, § 6.

## État
- [x] R1-0 sonde officielle : `core/crates/seed-bots/src/timeline.rs` + `examples/campaign_timeline.rs`
- [ ] Mesures « avant » (sonde, 3 graines × FR/EN/BU, IA + passif France)
- [ ] R1-1 renommée accumulée + actes à dates fixes (squelette, puis implémentation)
- [ ] R1-2 ambitions à effets
- [ ] Sauvegarde v12 + fixture
- [ ] ADR (remplace 0785, 0788 ; amende 0784, 0793 seuil)
- [ ] Mesures « après », seuil de victoire

## Coordination avec le lot TUTO (prestige de faction)
La renommée de maison sera `CampaignState::house_renown(data)` (entier, jauge) ; son journal est
`state.acts.renown` (faits : tour, source, points, cause). Le lot TUTO peut afficher un champ `prestige` de faction
provisoire ; à terme il sera remplacé par cette renommée (pas de doublon de règle dans `core/`).

## Prochaine étape
Mesures « avant » avec la sonde.
