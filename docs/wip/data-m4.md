# WIP — M4 données (personnages et dynasties)

Agent: data-model. Etat: en cours.

## Fait
- Schemas: trait.schema.json, skill.schema.json, names.schema.json créés.
- common.schema.json: ajout trait_id(existant)/skill_id/names_id, nouveaux EffectKind
  (siege_speed, construction_speed, diplomacy, intrigue, fertility, battle_charge,
  battle_ranged, battle_defense).
- character.schema.json: ajout "unborn" au statut.
- data/traits/ : 59 traits (tous les trait_* référencés + acquis du spec).
- data/skills/ : 30 compétences (10 par branche command/governance/court, tiers 1-3).
- data/names/ : 7 fichiers (fr,en,nl,oc,es,it,de), ~30 prénoms H/F chacun.
- Validation jsonschema: 0 erreur sur traits/skills/names/characters/factions.

## Prochaine étape
- core/crates/data-model: structs Trait, Skill, NameList, ids TraitId(existe)/SkillId,
  GameData.traits/skills/names, cross-ref traits/skills, deny_unknown_fields, tests.
- Characters: héritiers Navarre/Castille, enfants 1338-1346 (Charles V, Lionel, Jean de
  Gand, Louis d'Anjou, Jean de Berry, Philippe le Hardi, Philippe de Rouvres), liens
  famille bidirectionnels.
- docs/design/data-model.md: sections traits/skills/names/personnages.
- Supprimer ce fichier au commit final.
