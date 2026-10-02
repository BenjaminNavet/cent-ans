# GC3 — kits de l'Est et du Sud (maquettes de colonies)

Lot GC3 du chantier GC (ADR 0158, plan `docs/wip/gc-carte-generalisee.md`). Worktree
`../gp-gc-kit`, branche `feat/gc-kit`.

## But
Cinq familles d'architecture vers 1340 (`med`, `byz`, `rus`, `isl`, `steppe`), 5 types × 2
variantes : `game/assets/models/settlements/<type>_<famille>_<a|b>.glb`.

## Fichiers
- `tools/blender_scripts/settlements_east.py` : palette, aides (tour, courtine, coupole, yourte…),
  une fonction par famille et par type (`build_<famille>_<type>(variante)`), table `MODELS`.
- `tools/blender_scripts/settlements.py` : importe `settlements_east.MODELS`, budget par modèle.
- `tools/blender_scripts/settlements_sheet.py` : planche de contrôle d'une famille
  (`blender --background --python settlements_sheet.py -- <famille>`) →
  `docs/img/gc/kit_<famille>.png` (non suivi), lignes `SIZE <nom> <l> <p> <h> <triangles>`.

## Choix
- Maisons en primitives simples (pas les maisons du kit BR1) : peu d'éléments, gros, lisibles de
  loin, peu de triangles.
- Nouvelles teintes = matériaux unis de la palette, hors couches d'atlas : Godot garde leur couleur
  plate (`BuildingMaterials.remap_mesh` ne touche pas les noms inconnus). Les matières héritées
  (`Stone`, `Wood`…) passent toujours par l'atlas `Building`.

## État
- [x] Squelette : module, table des 50 modèles (bouche-trous), planche.
- [x] `med` (10 modèles, 612 à 1 612 triangles, planche regardée)
- [x] `byz` (10 modèles, 594 à 2 552 triangles, planche regardée ; coupoles de plomb et contreforts abaissés après coup)
- [x] `rus` (10 modèles, 692 à 2 900 triangles, planche regardée ; bardeaux assombris après coup)
- [ ] `isl`
- [ ] `steppe`
- [ ] Export des 50 `.glb`, budgets vérifiés, planches regardées.

## Prochaine étape
Famille `isl` (code écrit, planche à regarder), puis `steppe` ; à la fin, réexporter `med` et `byz` (faubourg factorisé, 8 maisons).
