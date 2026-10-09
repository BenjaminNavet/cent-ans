# 0276 — Actes royaux à recharge

Statut : accepté

## Contexte
Le prestige n'avait presque aucun puits (score, seuil de fondation d'ordre, rançon) et les factions jouaient toutes les
mêmes règles de personnage. Total War: Warhammer III offre des rites à recharge ; le XIVe siècle a ses équivalents :
sacre à Reims, lit de justice, Joyeuse Entrée, grande chevauchée, cour plénière, arrière-ban (`docs/wip/wh/personnages.md`
§ 3, point 1).

## Décision
- Données : `data/royal_acts/*.json` (schéma `royal_act.schema.json`) : coût en prestige et/ou livres, recharge et durée en
  saisons, factions autorisées (vide : toutes), gains immédiats (prestige, piété), effets du vocabulaire commun (`Effect`).
  Six actes sourcés.
- État : `FactionState::royal_acts` (dernier tour et fin des effets par acte, `#[serde(default)]`). Aucun état à
  incrémenter : l'actif se dérive de `state.turn < until`.
- Effets : fusionnés dans `research::faction_tech_effects` (économie, recrutement, mouvement, population les lisent déjà) ;
  les effets de bataille (`ArmyMorale`, `Battle*`) le sont aussi dans `skills::character_effects` pour les généraux de la faction.
- Ordre `Order::RoyalAct { act }` ; le prestige débité est celui du souverain (`change_ruler_prestige`), refus en français
  (recharge, prestige, trésor, faction).
- IA : `royal_acts::ai_choose_royal_act`, un acte par tour au plus, score selon l'état (guerre, désordre), réserve de trois
  fois le coût dans le trésor. Branchée dans `ai_minimal` et dans le crate `ai` (`plan_characters`).
- Interface : onglet « Actes royaux » du panneau de cour (`royal_acts_section.gd`), pont `get_royal_acts`.

## Conséquences
- Un puits de prestige et de livres, avec une identité par faction (France, Angleterre, Bourgogne) sans nouveau système.
- Ajouter un acte = ajouter un JSON ; aucune règle codée en dur.
- Les effets d'un acte ne distinguent pas les provinces : ils valent pour toute la faction.
