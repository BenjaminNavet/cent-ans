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

## Prochaine étape (PAUSE demandée par le joueur, 29/10)
Le workspace compile sans avertissement (`cargo check --workspace --all-targets`). Aucun test lancé depuis la réécriture.
1. Tests ciblés : `cargo test -p sim-campaign --test campaign_life acts::` (5 tests R1 + ambitions), `--lib acts::`
   (migration v11→v12), `seed-bots --test timeline`, `seed-bots --test ambitions` (R1-2, 4 × 3 actes × 3 maisons).
2. Fixture v12 : `CENT_ANS_WRITE_FIXTURE=1 cargo test -p game-api --test save_fixture -- write_fixture`, puis le test.
3. Sonde « avant » : relancer `BIN=out/r1/campaign_timeline_before CENT_ANS_DATA_DIR=out/r1/data_before/data
   tools/heavy.sh out/r1/run_probe.sh out/r1/before` (interrompue ; out/ non versionné, à recréer si perdu :
   binaire = sonde compilée sur 95b504043, données = `git archive 95b504043 data`).
4. Sonde « après » (`out/r1/after`), calibrage `renown_target` et points (passif ≤ 20, IA 25-45), seuil de victoire =
   médiane IA + 15, chiffres dans l'ADR XXXX ; INDEX déjà marqué (XXXX à numéroter à la fusion).
5. fmt, clippy, tests complets, seed bots rapides, passe longue en arrière-plan.

## Client (à afficher plus tard)
`ActView`/`TransitionView` gagnent `renown`, `house_renown`, `renown_recent` ; `ActPolicyRow` gagne
`renown_bonus_percent` ; deux ambitions à choisir (`policy_pending` tant qu'il en manque). Noms de champs inchangés.
