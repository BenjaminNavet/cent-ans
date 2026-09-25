# Lot NV1 — Batailles navales (à la Total War)

Branche `worktree-agent-a5b40232e9b8caf20`. ADR : `docs/decisions/0028-batailles-navales.md`.

## État
| Étape | État |
|---|---|
| 1. Cœur : `sim-battle::naval` (navires, tir, grappins, abordage, feu, brûlots, éperon, capture, naufrage), auto-résolution par phases, données `data/naval/` | fait, testé (`sim-battle/tests/nv1_naval.rs`, accord auto/3D 24/26) |
| 1b. Campagne : interception des traversées, réserves de navires, maîtrise de la mer, blocus (`sim-campaign::naval`) | fait, testé (`sim-campaign/tests/nv1_naval_campaign.rs`) |
| 2. Pont GDExtension (`NavalBattleSim`, méthodes navales de `CampaignSim`) | à faire |
| 3. Scène navale Godot | à faire |
| 4. Déclenchement depuis la carte + `--naval-scenario=sluys` | à faire |

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

## Pièges
- Disque presque plein (autres agents) : `core/target/release` supprimé ; éviter `--release`.

## Prochaine étape
Pont GDExtension `core/crates/godot-bridge/src/naval_sim.rs` + `campaign_sim_naval.rs`.
