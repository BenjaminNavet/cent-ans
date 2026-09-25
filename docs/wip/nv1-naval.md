# Lot NV1 — Batailles navales (à la Total War)

Branche `worktree-agent-a5b40232e9b8caf20`. ADR : `docs/decisions/0028-batailles-navales.md` (à écrire).

## État
| Étape | État |
|---|---|
| 1. Cœur : `sim-battle::naval` (navires, tir, grappins, abordage, feu, brûlots, éperon, capture, naufrage), auto-résolution par phases, données `data/naval/` | en cours : module compilé, scénario l'Écluse jouable sans rendu |
| 1b. Campagne : interception des traversées, flottes, maîtrise de la mer, blocus (`sim-campaign::naval`) | à faire |
| 2. Scène navale Godot | à faire |
| 3. Déclenchement depuis la carte + `--naval-scenario=sluys` | à faire |

## Fichiers
- Données : `data/naval/ships/ship_{cog,nef,galley,barge}.json`, `data/naval/rules.json`,
  `data/naval/fleets.json`, `data/naval/scenarios/sluys.json`.
- `core/crates/data-model/src/entities/naval.rs` (`ShipClass`, `NavalRules`, `NavalFleets`,
  `NavalData::load`), `ShipClassId` (`ship_`), `GameData::naval`.
- `core/crates/sim-battle/src/naval/` : `setup`, `ship`, `combat` (formules partagées), `sim`
  (`NavalSim`), `ai`, `auto` (`auto_resolve`), `outcome`, `scenario`.
- Tests : `core/crates/sim-battle/tests/nv1_naval.rs`.

## Prochaine étape
Équilibrage (pertes anglaises trop faibles à l'Écluse), tests (hauteur, feu, brûlot, éperon,
calibrage auto/3D), puis la campagne.
