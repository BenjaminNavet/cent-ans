# Lot R1 — renommée accumulée, actes à dates fixes, ambitions (audit 2026-10-29-campagne, R1-0/1/2)

Branche : `worktree-agent-adc1d3f985235a4e2` (worktree agent). Source : `docs/audits/2026-10-29-campagne.md` § 4 R1, § 6.

## État
- [x] R1-0 sonde officielle : `core/crates/seed-bots/src/timeline.rs` + `examples/campaign_timeline.rs`
- [ ] Mesures « avant » (sonde, 3 graines × FR/EN/BU, IA + passif France) — données d'origine extraites dans
      `out/r1/data_before/data` (le binaire « avant » refuse le nouvel acts.json), en cours
- [x] R1-1 renommée accumulée + actes à dates fixes (sim-campaign/src/acts.rs, accroches rançon, traité, recherche)
- [x] R1-2 ambitions à effets (données + schéma ; test seed-bots/tests/ambitions.rs)
- [x] Sauvegarde v12 + migration (fixture v12 à générer)
- [~] ADR brouillon docs/decisions/XXXX-r1-renommee-actes-a-dates-fixes-ambitions.md (mesures à compléter)
- [ ] Mesures « après », seuil de victoire

## Coordination avec le lot TUTO (prestige de faction)
La renommée de maison sera `CampaignState::house_renown(data)` (entier, jauge) ; son journal est
`state.acts.renown` (faits : tour, source, points, cause). Le lot TUTO peut afficher un champ `prestige` de faction
provisoire ; à terme il sera remplacé par cette renommée (pas de doublon de règle dans `core/`).

## Prochaine étape
cargo check workspace → tests acts, timeline, ambitions → fixture v12 → sonde « après » (out/r1/run_probe.sh out/r1/after)
→ calibrage renown_target / points → seuil victoire → ADR + INDEX.
