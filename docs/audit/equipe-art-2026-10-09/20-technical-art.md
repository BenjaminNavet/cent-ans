# Technical artist — état des lieux (09/10, lecture seule)

## 1. État actuel
- **Shaders** : 87 `.gdshader` + 41 `.gdshaderinc` (~14 200 l.) ; `battle_soldier_skinned` 1 120 l., `terrain` 930 l. (128 uniformes, 17 inclusions, aucune variante lointaine), `battle_soldier` 787 l.
- **Perf carte** (M4 Pro chargé, jamais sur machine calme) : plein écran 4096×2304 38-43 → 26 ms grâce au plancher 0,4 (ADR 0191) ; fenêtre 14-20 ms ; limité par les pixels ; terrain ≈ moitié de l'image (15,9 → 8,1 ms masqué), coût diffus.
- **Appels** : 500-1 400 par image (cible 2 500), 451/873 viennent des ombres ; le coût est en primitives (2,4-4,4 M Haute, 6,3 M Ultra). Végétation en MultiMesh (92 000 arbres en 15 appels).
- **Pics** : quadtree natif p99 38,4 → 34,5 ms ; restent `OutbuildingLayer._write_batches`, `life/reground` (≤ 10 ms), `qt/collect` (≤ 9 ms), `TownLayer` (~10 ms). `FrameBudget` 6 000 µs.
- **Bataille** : simulation p99 ~1,1 ms ; poses et tampons 3-4 ms de `_process`.
- **Budgets d'assets** : bible § 14.4-14.5 ; `dn_ingest` vérifie via `dn_ingest_classes.json` (sortie 2 au dépassement).
- **Qualité** : 4 préréglages, MetalFX, budget pixels HiDPI (ADR 0123).

## 2. Forces
- Mesure solide (A/B dans le même processus `--bench-ab`, sondes par système).
- Instancing massif, imposteurs, météo cuite, quadtree Rust.
- Bonne factorisation par inclusions ; ingestion déterministe avec budgets en données.

## 3. Faiblesses
1. **Textures DN triplées** : chaque LOD extrait sa propre `_Image_0.jpg` (md5 identiques), 1 390 doublons ; VRAM doublée si deux LOD chargés.
2. **Compression non garantie** : 5 977/6 059 imports en sans perte ; `detect_3d/compress_to=1` ne bascule jamais hors éditeur → risque d'export non compressé.
3. Double LOD : `generate_lods=true` sur 2 085 glb en plus des LOD manuels ; 18 scripts seulement utilisent `visibility_range` ; `create_shadow_meshes` partout.
4. Shader terrain monolithique, sans variante par distance — dernier levier sur le temps d'image.
5. Code dupliqué : 24 fichiers redéfinissent `hash`/`noise` ; 43 shaders sans inclusion ; soldats 1 900 l. à tronc commun partiel.
6. `719.0` codé en dur 65 fois au lieu de `MapScale`.
7. **Outils de mesure supprimés** par SC (`a4b5353ea` : `a6_drawcalls_probe.gd`, `pb1_bench.gd`, `hc_density_probe.gd`) mais cités par les docs ; aucun banc de régression ni budget ms par système.
8. MultiMesh pleine carte (~1 M primitives récupérables), ombres non réglées, Ultra jamais mesuré.

## 4. Améliorations
| # | Action | Impact | Effort | Dépend de |
|---|---|---|---|---|
| 1 | Forcer compression VRAM + mipmaps sur toutes les textures 3D, contrôle CI | VRAM, chargement, export | S | build |
| 2 | Une texture par modèle DN partagée par les 3 LOD ; couper `generate_lods` | Centaines de Mo | M | pipeline, SC |
| 3 | Variante lointaine du shader terrain (`#define`) ou passe demi-résolution, A/B | −5 à −10 ms estimés | L | RV, carte |
| 4 | Remettre sonde appels/primitives + banc de régression (budgets ms en JSON) | Anti-régression | M | QA, SC |
| 5 | MultiMesh découpés par tuile, `shadow_range` maquettes, cascades relief | ~−1 M primitives | M | carte |
| 6 | Étaler/porter en Rust `_write_batches`, `qt/collect`, `TownLayer` | Moins de saccades | M-L | Rust |
| 7 | Inclusions communes bruit/hash/soldats | Maintenance | S-M | bataille |
| 8 | Remplacer les `719.0` par `MapScale` + test | Robustesse | S | carte |
| 9 | Mesures Ultra et pics sur machine calme, plein écran | Fiabilité | S | machine |

Coût : nul partout.
