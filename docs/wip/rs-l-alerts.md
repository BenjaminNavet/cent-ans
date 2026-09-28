# RS-L — Détail du siège dans `get_provinces_snapshot` — fichier de reprise

Branche `feat/rs-l-alerts`, depuis `main` (48f1a5f8). Suite de la note E (`docs/wip/restes.md`,
`docs/wip/rs-e-map.md` point 2) : `game/scripts/ui/alerts.gd::collect` gardait un appel
`get_province_state` par province assiégée pour lire le détail du siège (`attacker`, `supplies`,
`turns_elapsed`), absent de `get_provinces_snapshot`. Cible privée `core/target-rs-l`.

## Plan

1. `core/crates/godot-bridge/src/campaign_sim_provinces.rs` : ajouter à `get_provinces_snapshot`
   trois tableaux groupés `siege_attacker` (PackedStringArray), `siege_supplies`
   (PackedInt32Array), `siege_turns_elapsed` (PackedInt32Array), mêmes noms que dans le dict de
   `get_province_state`'s `siege`. Vides/0 si pas de siège.
2. `game/scripts/map/province_snapshot.gd` : lire ces 3 champs dans `ProvinceSnapshot.read`
   (chemin groupé) ; chemin de secours (`get_province_state`) inchangé pour compat.
3. `game/scripts/ui/alerts.gd` : remplacer le second appel `sim.call("get_province_state", ...)`
   par une lecture directe de `snapshot.siege_attacker/siege_supplies/siege_turns_elapsed`.
4. Test Rust du pont sur les nouveaux champs (`campaign_sim_provinces.rs` ou test existant du
   module). `smoke.gd`, tests d'alertes/carte existants.

## État

- [ ] Rust : champs ajoutés + test.
- [ ] `province_snapshot.gd` : lecture groupée.
- [ ] `alerts.gd` : appel par province supprimé.
- [ ] Tests Godot verts.

## Prochaine étape

Commencer par le champ Rust (squelette + test), commit wip, puis le pont Godot.
