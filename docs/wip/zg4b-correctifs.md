# ZG4b — correctifs de la vue rapprochée relevés en recette (Q3)

Worktree d'agent (depuis `main`, refusionné après ZG6 7f38c532). Liens symboliques non versionnés
`data/map/pyramid`, `tools/geo/raw`. Dylib : `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target`.
Rendu seulement. Captures : `godot --path game res://scenes/start_menu.tscn -- --autostart=fac_england
--screenshot=… --focus=2018,1486.6,<d>` (Londres ; joueur anglais, sinon le brouillard de guerre voile Londres).

## Diagnostic
1. **Sol beige nu de près à Londres** : trois causes cumulées.
   - brouillard matinal de la météo CM2 (`weather_ground`) : nappe grise jusqu'à 80 % peinte sur le
     sol, qui efface textures et parcellaire de près → atténuée de près (`weather_mist_near`) ;
   - parcellaire ZG5b coupé par `hc <= 0.5` alors que ZG3b relève les terres basses (rives de la
     Tamise) au plancher 0,5 m → test `hc < 0.25` ;
   - maquette emblématique masquée au palier site et rien de fin dans sa zone → plancher de caméra
     `landmark_min_distance` (2,6) dans `close_camera.tres`, levé par VH4.
   - (joueur français : le voile beige est le brouillard de guerre, normal.)
2. **Ruban rouge et gris** : `TradeRouteLayer` (C5), pas un pont. `refresh()` force
   `_wanted_visible = true` même couche Commerce éteinte (appelé par `refresh_all`) : routes actives
   (sépia) + route coupée Calais-Londres (grise) par-dessus. Non corrigé (autre session).
   Même famille : `PathPreview` (aperçu de chemin d'armée, orange, ~1 km) reste à l'échelle carte
   aux paliers vallée/site.

## État
- [x] reproduction, identification
- [x] plancher déclaratif + atténuation de la brume de près + parcellaire des terres basses
- [ ] test zg4 (plancher), ponts-portes / pic de basculement
- [ ] tests, captures `docs/img/zg4b/`, doc `godot-map.md`, addendum ADR

## Prochaine étape
Test du plancher dans `zg4_camera_test.gd`, puis ponts-portes (échelle) et étalement du basculement.
Retirer les options temporaires `ZG4BTMP` de `campaign_map.gd` (non commitées).
