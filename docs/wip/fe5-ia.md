# FE5 — IA féodale (`feat/fe5-ia`, worktree `../game_project-fe5`)

Spec FE § 4.2-4.4, 4.6, 5 ; plan section F5 ; ADR 0110. `CARGO_TARGET_DIR=core/target-fe5`.

## État
- [x] Données : `data/ai/feudal.json` + `data/schemas/ai_feudal.schema.json` (+ test pytest),
      `AiFeudal` (data-model, `GameData::ai_feudal`) ; `rank_strategies.county` dans `doctrines.json`.
- [x] Cœur : `feudal/policy.rs` (`FeudalPolicy`, `install_policy`, `PROVISIONAL`), `feudal/acts.rs`
      (`revolt`, `switch_allegiance`), `answers_host`, ordres `DeclareCommise`, `GrantTitle`, `Revolt`,
      `SwitchAllegiance` ; félonie ouverte avant la rupture du lien (révolte).
- [x] IA : `ai/src/feudal.rs` (protection, arbitrage, ost, commise, révolte, hommage, concession,
      survie d'abord, titres exigés), branché dans `campaign.rs::state_plans`.
- [x] Tests `ai/tests/feudal_ai.rs` (7) ; `g4.rs` réactivé (vassal courtisé hors suzerain).
- [ ] Suite complète verte, banc `pb1_core` après.

## Banc
`cargo run --release -p ai --example pb1_core 12 1 2 3` (part Rust de PB1 : fins de tour complètes).
Référence `feat/fe` a9f7b409 : 194-230 ms moyenne (bruit machine). Plafond 1,5× ≈ 300 ms.

## Prochaine étape
Suite complète, mesure pb1_core, commit, rapport.
