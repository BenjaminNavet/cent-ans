# UX5-R — idées des cinq écrans qui exigent des règles en Rust (reprise ultérieure)

Origine : chantier UX5 (2026-10-10, `docs/wip/ux5-ecrans.md`), fusionné dans main (18df5ac1d).
Les lots Godot sont faits ; ce qui suit demande une règle dans `core/` (sim-campaign) puis une
exposition dans le pont (`core/crates/godot-bridge`) avant l'UI. Estimation globale : une nuit de
lots (≈ 6-8 lots, agents `cent-ans-dev` pour la règle, `cent-ans-mech` pour l'UI).

Règles communes : chaque règle a ses données dans `data/` (validées par `data/schemas/`), un test
Rust, et un ADR quand elle change l'équilibre. Charte UI : celle de `ux5-ecrans.md` § Charte commune.

## Lots proposés (ordre conseillé)

| # | Écran | Idée | Ancrage actuel | Taille |
|---|-------|------|----------------|--------|
| R1 | Savoirs | Réordonner la file (glisser un sceau du ruban) | `research.rs` : `research_queue` (l.505-551, ajout/retrait seulement) ; pont `campaign_sim_tech.rs` `get_research_queue` | S |
| R2 | Diplomatie | Aperçu des alliés qui suivront une déclaration de guerre | appels d'alliés à la déclaration (sim-campaign, guerre) ; rien d'exposé avant l'ordre | M |
| R3 | Diplomatie | Contre-offres multiples (2-3 variantes proposées par l'IA) | `treaty_explain.rs` l.160-200 : une seule variante « sans « X » » au-dessus de `ACCEPT_CHANCE` | M |
| R4 | Diplomatie | Journal des parjures (réputation lisible) | `negotiation.rs` `TreatyRecord.rupture: Option<Rupture>` (l.375) : la donnée existe, il manque un agrégat par faction + effet nommé | S |
| R5 | Unités | Prévision d'usure (pertes attendues au prochain tour) | `economy.rs` `resolve_attrition` (l.1005) : extraire une fonction pure de prévision | S |
| R6 | Unités | Fusion d'osts sur la même case + actions rapides par ligne | pas d'ordre de fusion ; ordres existants dans le pont | M |
| R7 | Recrutement | Arrière-ban (ordre `Muster` : levée gratuite lente, coût en ordre public) | `map_scenes.rs` n'a que la scène visuelle `SceneKind::Muster` | M-L |
| R8 | Recrutement | Contres d'unités (champ `counters` en données) + entretien projeté avec revenu net | `data/` unités ; `get_recruitable` (`campaign_sim.rs`) | S |
| R9 | Colonies | Ordre public par colonie + durée totale du chantier ; regroupement par seigneur ; actions en masse | `campaign_sim_settlements.rs` ; `holdings_controller.gd` | M |
| R10 | Savoirs | Eurêkas médiévaux (un fait de jeu accélère un savoir) | données techs + hooks d'événements | L (conception) |
| R11 | Diplomatie | Négociation à tours (l'IA répond par une contre-proposition, plusieurs échanges) | `negotiation.rs` | L (conception) |
| R12 | Unités | État « en voyage » (armée en mer entre deux tours, glyphe embarqué) | traversées résolues dans le tour (ADR 0167), `ArmyPosition` = `Field`/`Settlement` seulement ; pont `embarked = false` exact aujourd'hui ; `unit_roster_controller.gd` gère déjà le glyphe | L (conception, change le rythme naval) |

Tailles : S ≈ 30 min agent, M ≈ 1 h, L = conception d'abord (ADR avant code).

## Ce que chaque lot doit livrer
- Règle + test Rust (`cargo test -p sim-campaign`), `cargo clippy -- -D warnings`.
- Lecture/ordre exposé dans le pont, puis UI dans l'écran UX5 concerné
  (`diplomacy_*.gd`, `tech_panel.gd`/`tech_tree_view.gd`, `recruit_basket.gd`,
  `holdings_controller.gd`, `unit_roster_controller.gd`).
- Test Godot headless `game/tests/ux5r_<n>_test.gd`.
- R7, R10, R11 : ADR d'abord (équilibre / conception), pas de code avant.

## État
- [ ] Rien commencé. Reprendre par R1, R4, R5, R8 (petits, indépendants), puis R2, R3, R6, R9.
