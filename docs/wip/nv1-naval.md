# Lot NV1 — Batailles navales (à la Total War)

Branche `worktree-agent-a5b40232e9b8caf20`. ADR : `docs/decisions/0028-batailles-navales.md`.

## État
| Étape | État |
|---|---|
| 1. Cœur : `sim-battle::naval` (navires, tir, grappins, abordage, feu, brûlots, éperon, capture, naufrage), auto-résolution par phases, données `data/naval/` | fait, testé (`sim-battle/tests/nv1_naval.rs`, accord auto/3D 24/26) |
| 1b. Campagne : interception des traversées, réserves de navires, maîtrise de la mer, blocus (`sim-campaign::naval`) | fait, testé (`sim-campaign/tests/nv1_naval_campaign.rs`) |
| 2. Pont GDExtension (`NavalBattleSim`, méthodes navales de `CampaignSim`) | fait |
| 3. Scène navale Godot (mer, navires, équipages V2, volées BV1, grappins, passerelles, feu, naufrage, HUD, fin) | fait, captures `docs/audit/captures/nv1/` |
| 4. Déclenchement depuis la carte (`NavalCampaign`, `NavalPreBattleDialog`) + `--naval-scenario=sluys` | fait, test `game/tests/nv1_naval_test.gd` |
| 5. Finitions : captures feu (La Rochelle), smoke, fusion main | en cours |

## Fichiers
- Données : `data/naval/ships/ship_{cog,nef,galley,barge}.json`, `data/naval/rules.json`,
  `data/naval/fleets.json` (réserves, mers, fusiliers marins, noms des mers),
  `data/naval/scenarios/{sluys,la_rochelle}.json` ; schémas `ship_class`, `naval_rules`,
  `naval_fleets`, `naval_scenario` ; test `tools/tests/test_naval_schemas.py`.
- `core/crates/data-model/src/entities/naval.rs` (`ShipClass`, `NavalRules`, `NavalFleets`,
  `Marines`, `NavalData::load`), `ShipClassId` (`ship_`), `GameData::naval`.
- `core/crates/sim-battle/src/naval/` : `setup`, `ship`, `combat` (formules partagées), `sim`
  (`NavalSim`), `ai`, `auto` (`auto_resolve`), `outcome`, `scenario`.
- `core/crates/sim-campaign/src/naval.rs` : `NavalState` (champ `naval` de `CampaignState`,
  `serde(default)`), `intercept` appelé par `order_embark` (`march.rs`, fin de traversée extraite
  dans `land_crossing`), `resolve_season` (blocus, déclin, chantiers) dans `turn.rs`.
- Test existant retouché : `campaign.rs::sea_crossings_go_port_to_port_and_take_the_turn` vide
  les réserves navales pour garder une traversée sans interception.

- Godot : `game/scripts/naval/` (`naval_scene`, `naval_ship_view`, `naval_sea`, `naval_waves`,
  `naval_fire`, `naval_effects`, `naval_hud`, `naval_pre_battle_dialog`, `naval_campaign`),
  `game/shaders/naval_{sea,oar}.gdshader`, `game/scenes/naval/naval_battle.tscn`, modèles
  `game/assets/models/naval/*.glb` (`tools/blender_scripts/naval_ships.py`).
- Retouches partagées minimales : `campaign_map.gd::_offer_pending_battles` (appel
  `NavalCampaign.offer`), `start_menu.gd` (`--naval-scenario=`).
- Commandes : `godot --path game res://scenes/naval/naval_battle.tscn -- --naval-scenario=sluys
  [--screenshot=<png> --shot-at=375 --camera=melee|close|overview|deck] [--benchmark --bench-at=360]`.
- Mesure : 26 navires, ~2 000 figurines, 45 000 traits : 60 i/s (plafond vsync du pilote macOS),
  scène 1,2 ms CPU par image, rendu CPU 0,4 ms.

## Pièges
- Disque presque plein (autres agents) : `core/target/release` supprimé ; éviter `--release`.

## Prochaine étape
Capture du feu (scénario `la_rochelle`), capture de l'écran d'avant-bataille, fusion de main,
vérifications finales (fmt, clippy, tests, build.sh, pytest, import, smoke).
