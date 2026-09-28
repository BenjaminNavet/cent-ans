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
- [x] Suite complète verte (fmt, clippy, `cargo test`), pytest des schémas IA.
- [x] Banc : pas de surcoût mesurable (voir ci-dessous) ; évaluation allégée non activée (`light_evaluation.enabled = false`).

## Banc
`cargo run --release -p ai --example pb1_core 12 1 2 3` (part Rust de PB1 : fins de tour complètes).
Référence `feat/fe` a9f7b409 : 194-230 ms moyenne (bruit machine). Plafond 1,5× ≈ 300 ms.

Après FE5, mesures appariées (alternées) sur machine chargée (load ≈ 110, autres sessions) :
après 774 / 714 / 1359 / 1542 ms, référence 644 / 947 / 1517 / 1481 ms : somme 4389 contre 4588, ratio ≈ 0,96.

Sonde 40 tours, graines 1-2 : 21 cas de félonie (surtout « allié de l'ennemi » : Albret envers la France,
Bourgogne et Bohême envers l'Empire), 2 changements d'allégeance, 20 réponses à l'ost, aucune commise
(les félons sont adossés à l'Angleterre : rapport < 2) ni appel de protection.

## Points ouverts (F8)
- Aucune commise en 40 tours : la commise de Guyenne (déclencheur historique) reste à régler en F8
  (`commise.min_power_ratio`, `max_wars`).
- Pont Godot : `ai::feudal::install()` dans `CampaignSim::init` (non testé en jeu, pas de Godot ici).
- Le banc PB1 Godot (`game/tests/pb1_turns.gd`) n'a pas été lancé : mesure sur la part Rust (`pb1_core`).

## Prochaine étape
Revue et fusion dans `feat/fe` par l'orchestrateur.
