# Lot F1 « Règles inertes » — état

Branche : `worktree-agent-ae66d663ee24e14ab`. Tests : `core/crates/sim-campaign/tests/f1_effects.rs`.

## Points
1. [x] Bâtiments : `Garrison`, `RecruitCost`, `Supply`, ciblage par classe (et par catégorie d'unité).
2. [ ] Technologies : `army_upkeep`, `army_experience`, `recruit_cost`, `movement`, `production`,
   `siege_resistance`, `fortification_level`, `wealth`, `prestige` ; surplus de recherche conservé.
3. [ ] Traits/compétences : `research_civil`/`research_military`, `Diplomacy`, `Intrigue`, `Loyalty`.
4. [ ] Armées alliées de la province dans la bataille (auto-résolution + `battle_setup`).
5. [ ] Événements : capture (+ rançon), effets différés (chaînes), Charles VI + Jeanne de Bourbon, folie.
6. [ ] `docs/status.md` (Limites connues) + paragraphes « F1 » des specs.

## Choix (point 1)
- Ciblage : un effet avec `class` ne touche que cette classe (`EffectTotals::classes`), un effet avec
  `unit_category` que cette famille d'unités (`EffectTotals::unit_categories`) ; les champs de premier
  niveau ne gardent que les effets non ciblés. Le pont expose `effects.by_class`.
- `Garrison` N : les N unités de garnison les plus chères ne coûtent rien ; la garnison d'une ville tenue
  par son propriétaire et non assiégée regagne 5 % × N de son effectif max par saison (plafond 50 %).
- `RecruitCost` : province (bâtiments + gouverneur) + techs de la faction, global ou par famille ;
  pourcentages additionnés, plancher 25 % du prix de base.
- `Supply` : en territoire ami, bâtiments de la province + `Supply` fixe du général ajoutés à la
  récupération (40) ; hors territoire ami, `Supply` % et `AttritionResistance` du général réduisent la perte.

## Prochaine étape
Point 2 (technologies).
