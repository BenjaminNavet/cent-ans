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
- [x] Données, cœur, game-api, client, textes : écrits, clippy cœur et client propres, tests du lot verts.
- [x] Smoke `campaign_turns` vert (dump `season_story.story.why` non vide au tour 6 ; au tour 3 : 3 causes de variation du trésor).
- [ ] Seed bots rapide (lancé), ADR à numéroter à la fusion.
- Échecs existants hors lot : `campaign_life::acts::reign_score_moves_until_1453` (échoue aussi sans `season_marks::record`), `seed-bots ambitions` (`en2_amb_guyenne`), client `markers::dn::real_table_and_glbs` (GLB `city_west` absent).

## Points ouverts
- Termes de prestige / hommes / troubles : captures au début du tour (`season_marks`), les contributions de la prévision décrivent la saison jouée.
