# XP-POURQUOI (R4) — la cause montrée au moment de l'effet

Spec : `docs/audits/2026-10-29-experience.md` § R4 et § « Lot XP-POURQUOI ».

## Plan
1. Données : `data/rules/cause_thresholds.json` + schéma + `data_model::CauseThresholds` (fait).
2. Cœur : 3 `CauseKind` (`TreasurySwing`, `UnrestSwing`, `AttitudeShift`), constructeurs dans `cause_builders.rs`,
   module `season_marks.rs` (repères de saison + détection des variations, appelée par `turn.rs`).
3. `game-api` : `causes::season_story`, `hud::TopBar.figures` (écart + 3 termes par chiffre), textes `causes.fr.json`.
4. Client : écart sous les chiffres + infobulle (`bar.rs`, `text.rs`), Récit (`report.rs`), fil sans coupure dans un nombre (`feed.rs`), dump `season_story`.
5. Tests, fumée, seed bots (invariant `max_per_season_player`), ADR, `docs/architecture.md`.

## État
- [x] Données + entité data-model.
- [ ] Cœur, game-api, client, tests.

## Points ouverts
- Termes de prestige / hommes / troubles : captures au début du tour (`season_marks`), les contributions de la prévision décrivent la saison jouée.
