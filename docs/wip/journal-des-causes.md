# Journal des causes (ADR 0748)

Branche : worktree-agent-a810e54a95d255ac3.

## État
- Fait (core) : `sim-campaign::causes` (types, journal borné 300, `harvest`, migration v9 -> v10,
  `STATE_VERSION` 10), `cause_builders` (une fonction pure par type, testée), producteurs câblés :
  bataille (joueur impliqué, `movement/battle.rs`), place prise (`siege::capture`), banqueroute
  (`economy.rs`), révolte et grogne (`revolt.rs`, termes du mécontentement via
  `population::province_unrest_terms`), guerre déclarée + raison IA (`diplomacy/war.rs`,
  `diplomacy/ai.rs::war_reason`), alliance rompue / appel refusé, succession (`characters.rs`), fin de
  partie (`victory.rs`).
- Fait (données) : `data/ui/causes.fr.json` + `data/schemas/causes.schema.json` (+ `test_schemas.py`).
- Fait (game-api) : `causes.rs` (vues, filtre, `top_causes`, `last_battle_causes`, `notification_tooltip`),
  `JournalEntry::cause_id`, fixture de sauvegarde v10 (`save_v10.json`, v9 conservée).
- À faire : client (onglet Causes du panneau Chronique, fin de bataille, fin de partie, infobulles),
  dump d'état, tests client, smoke, seed-bots rapide.

## Prochaine étape
Client Bevy : `client/src/campaign/panels/chronicle/` (onglet), écrans de fin (`flow`), infobulles du fil
d'événements (`hud/feed.rs`).

## Points ouverts
- Doctrine de recrutement de l'IA (crate `ai`) non journalisée : le planificateur lit un état immuable.
- Successions d'héritage féodal (`feudal/inherit.rs`) sans cause.
- Cause de bataille : pas de « cavalerie chargée de flanc » (le `BattleResult` de campagne ne la porte pas).
