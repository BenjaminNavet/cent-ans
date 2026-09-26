# SZ6 — pics d'images côté scripts sur la carte de campagne (suite S6 de ZG7c)

Branche `feat/sz6-script-spikes` (worktree d'agent, depuis `main` 7e1ac032, `main` c4064c29
fusionné). Liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal.
Aucun changement Rust.

Objectif : p99 des images < 50 ms sur le parcours `--bench-map`, sans changement visuel. **Atteint.**
Coordination : sélection du quadtree en Rust = lot PB3g (après fusion de SZ6) ; pas d'ADR écrit
(application de l'ADR 0051 ; 0082 serait le prochain libre) ; profils Cargo non touchés (PB3a).

## Outils
- `godot --path game res://scenes/campaign_map.tscn -- --stage=map --hide-armies --bench-map
  --bench-probe` : `PerfProbe` (`game/scripts/dev/perf_probe.gd`) ; rapport `probe` : par section,
  temps cumulé dans les pics > 50 ms, pics dominés, pire durée ; 12 pires images. Sections
  permanentes : `map.*` (`CampaignMap._process`), `lod/*` (`update_lod`), `qt/*` (étapes du quadtree).
  Pendant l'analyse, les `_process` des autres nœuds étaient enveloppés temporairement (non commité).
- A/B entrelacé : copies de `game/` par clonage APFS (`cp -cR`, sans place disque) dans le
  scratchpad, scripts de `main` d'un côté, de la branche de l'autre, passes alternées.
- Test : `godot --headless --path game --script res://tests/sz6_spikes_test.gd`.

## Diagnostic (sonde, charge 80-150)
1. `RoadRenderer.update_view` : ≥ 1 ruban drapé par image, jusqu'à 115 ms (195 pics sur 212).
2. `Vegetation._apply_lod` : `MultiMesh.mesh = …` après `buffer` → relecture GPU synchrone du
   tampon par le serveur de rendu (boîte englobante) : 74-86 ms.
3. `LandmarkModel` : recuisson en tranches de 1,5 ms par maquette et par image : 16-31 ms.
4. Changements de niveau des morceaux (jusqu'à 21 dans une image, 51 ms d'écouteurs : colonies,
   ponts) ; étiquettes des colonies recalculées toutes (570) à chaque image d'un zoom (≤ 13 ms) ;
   `LifeEffects._reground` sur tous les points (≤ 29 ms).

## Correctifs (tous commités)
- Rubans de route dans des fils (`RoadRenderer.RibbonJob`, instantané `surface_snapshot`).
- Végétation : MultiMesh recréé depuis la copie processeur du tampon (`_with_mesh`).
- Maquettes : recuisson complète dans un fil (`_bake_rows`).
- `TerrainBuilder.level_emit_budget_ms` (4 ms, plus proches d'abord, ≥ 1 par image ; tous hors
  image : `FrameBudget.in_frame`).
- Étiquettes : par morceau recalé ; en entier au changement d'échelle verticale ou de palier près.
- `LifeEffects._changed_points` : index des points par morceau.

## Mesures finales (4 passes alternées, main c4064c29 / branche, médianes, charge 75-150)
| | p50 | p99 | pire | > 50 ms | descente p99 | scripts p99 |
|---|---|---|---|---|---|---|
| main | 19,7 | 91,1 | 182 | 231 | 93,3 | 86,9 |
| SZ6 | 18,5 | 38,1 | 52 | 3 | 37,8 | 29,4 |
Une passe SZ6 sur quatre à 92 images > 50 ms dont 7 dominées par les scripts (bruit machine / GPU).

## Limites, suites
- Pics résiduels (sonde après correctifs) : `qt/select` / `qt/apply` / `qt/collect` 9-12 ms sous
  charge (→ PB3g, quadtree en Rust) ; `Vegetation._start_ground_jobs` jusqu'à 70 ms une fois :
  `request_reground` recopie en Rust (`to_vec`) toutes les pages de l'instantané du morceau (jusqu'à
  256 × 512 Ko) → piste PB3 : magasin de pages partagé côté Rust (copie une fois par page) ;
  `NextHintController.refresh` 15 ms une fois par seconde (appels au cœur) ; `TownLayer` ≤ 10 ms.
- Rubans de route, recuissons de maquettes et niveaux de morceaux arrivent une à quelques images
  plus tard (ADR 0051).

## Prochaine étape
Lot terminé : fusion par l'orchestrateur.
