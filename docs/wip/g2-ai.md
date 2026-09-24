# Lot G2 « IA : alignement historique » — état

Branche : `g2-ai-historical`. Périmètre : `core/crates/ai`, IA de `sim-campaign/src/diplomacy.rs`
(`plan_diplomacy` et assistants), `ai_personality` de `data/factions/*.json`, ajout `Order::Subsidy`.

## Points
1. [ ] Subsides : allié riche → allié endetté en guerre contre le même ennemi (France → Écosse). Cible < 3 banqueroutes/décennie pour l'Écosse.
2. [ ] Pays-Bas : Flandre ruinée par l'embargo penche vers l'Angleterre ; Hainaut/Brabant dans la coalition anglaise 20-60 % des tours de guerre.
3. [ ] Bourgogne opportuniste (Troyes) quand l'Angleterre domine.
4. [ ] Trésors dormants ≤ 12 saisons (médiane ≤ 8) après 1350.
5. [ ] Docs (`status.md`, `m9-ai.md` § G2), tests `ai/tests/g2.rs`, 0 % refus France dans `ai_probe`.

## Prochaine étape
Mesure de référence avec `century_probe`, puis point 1.
