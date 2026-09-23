# M4 — Personnages et dynasties : état d'avancement

Démarré 2026-09-23. Agent: sim-campaign implementation.

## Constat data-model
`git log -- core/crates/data-model data` ne montre aucune activité de l'agent data
(seul commit lié : "docs: M4 characters and dynasties specification"). `data.traits`,
`data.skills`, `data.names`, `EffectKind::{SiegeSpeed,ConstructionSpeed,Diplomacy,
Intrigue,Fertility,BattleCharge,BattleRanged,BattleDefense}`, `CharacterStatus::Unborn`
n'existent pas dans `core/crates/data-model` au moment du démarrage (vérifié par lecture
directe des fichiers). Interdiction de toucher `data-model`/`data/` : implémentation de
tables internes (traits/compétences/prénoms) dans `sim-campaign` en attendant, à
reconnecter plus tard (voir rapport final).

## Plan
1. state.rs: CharacterState nouveaux champs, STATE_VERSION=3.
2. skills.rs (nouveau): XP, learn_skill, tables traits/compétences internes, character_effects.
3. dynasty.rs (nouveau): mariages, gouverneurs, naissances, morts+traits, succession, régence, queries bridge.
4. orders.rs: LearnSkill, AssignGovernor, ProposeMarriage, DebugGrantXp.
5. battle_auto.rs/movement.rs: effets du général en bataille, XP, traits acquis.
6. buildings.rs: EffectTotals étendu, province_effects + gouverneur.
7. siege.rs: SiegeSpeed, siege_master.
8. setup_1337.rs: init nouveaux champs.
9. turn.rs: naissances hiver, XP gouvernance, majorité/régence.
10. tests/m4.rs: >= 14 tests.

## Statut
Étapes 1-10 faites ; 16 tests M4 (tests/m4.rs). Pont GDExtension fait.
désormais (commit 4bd1316) et sont utilisés directement. Prochaine étape : 9 (turn.rs), puis 10 (tests/m4.rs), puis pont GDExtension (§ 3).
