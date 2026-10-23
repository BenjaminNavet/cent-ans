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
- Fait (client) : onglet Causes du panneau Chronique (`CausesRequest`, filtre, facteurs dépliables, dump
  `panel_chronicle.causes`), bouton « ? » et infobulle sur le fil d'événements (`hud/feed.rs`), bloc
  « trois causes principales » + bouton Causes sur l'écran de fin (`ending.rs`, `EndView.causes`),
  bouton « Pourquoi cette bataille ? » sur l'écran de fin de bataille, hook `CENT_ANS_CAUSES_DEMO=1`.
- Testé : cargo fmt/clippy core et client propres ; `cargo test --workspace` core OK (hors course
  de fixtures corrigée ensuite) ; tests client ciblés chronicle/feed OK.
- Non fait : smoke, dump d'état et captures du client (arrêt demandé), seed-bots, clippy client final
  après les dernières retouches.

## Prochaine étape
Intégrateur : clippy client, smoke, dump `panel_chronicle.causes` avec `CENT_ANS_CAUSES_DEMO=1`.

## Points ouverts
- Doctrine de recrutement de l'IA (crate `ai`) non journalisée : le planificateur lit un état immuable.
- Successions d'héritage féodal (`feudal/inherit.rs`) sans cause.
- Cause de bataille : pas de « cavalerie chargée de flanc » (le `BattleResult` de campagne ne la porte pas).
