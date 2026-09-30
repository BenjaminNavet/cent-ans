# Textures des figurines fines — provenance

- `fine_atlas_lod0.png`, `fine_atlas_lod1.png`, `fine_detail.png` : cuits par le projet (lot FG3,
  ADR 0088 ; `tools/blender_scripts/battle_fine_bake.py`, `battle_fine_tiles.py`).
  `fine_detail.png` n'est chargé qu'avec `--no-ga1`.
- `fine_horse.png` : pelage de cheval CC0 réduit (ADR 0088).
- `fine_detail_ga1.png` (12 couches 512² : RG normale, B relief, A rugosité) et
  `fine_detail_albedo.png` (12 couches 256², albédo de détail centré). Couches 0-7 : **scans
  ambientCG, CC0 1.0** (lot SR1, voir ci-dessous) ; couches 8-11 (peau, cheveux, robes) :
  **générées, projet** (lot GA1, ADR 0104). Images sources `openai/gpt-5-image-mini` via OpenRouter, prompts dans
  `data/art/materials.yaml` (ordre = couches), chaîne `tools/cent_ans_tools/material_gen.py`
  (`cent-ans assets materials`, puis `material_gen.build_fine_arrays(<dossier>)`). Images
  brutes non versionnées. Coût consigné dans `docs/budget.md` (section GA).

## Couches scannées (lot SR1) — ambientCG, CC0 1.0

Source : [ambientCG](https://ambientcg.com) (Lennart Demes), licence
[CC0 1.0](https://docs.ambientcg.com/license/) ; archives `1K-JPG` obtenues par l'API v2
(`https://ambientcg.com/api/v2/full_json?id=<Id>&include=downloadData`), brutes hors dépôt
(`~/dev/cent-ans-raw/sr1/<Id>/`). Cartes utilisées : Color, NormalGL, Roughness, Displacement.

| Couche | Asset | URL |
|---|---|---|
| 0 WOOL | Fabric045 | https://ambientcg.com/a/Fabric045 |
| 1 LINEN | Fabric061 | https://ambientcg.com/a/Fabric061 |
| 2 FUSTIAN | Fabric066 | https://ambientcg.com/a/Fabric066 |
| 3 GAMBESON | Fabric048 | https://ambientcg.com/a/Fabric048 |
| 4 MAIL | Chainmail002 | https://ambientcg.com/a/Chainmail002 |
| 5 LEATHER | Leather033A | https://ambientcg.com/a/Leather033A |
| 6 PLATE | Metal055A | https://ambientcg.com/a/Metal055A |
| 7 WOOD | Wood049 | https://ambientcg.com/a/Wood049 |

Modifications : scan entier (déjà tuilable) réduit de 1024² à 512² (normale renormalisée,
pente multipliée par `normal_strength`), rugosité décalée (`roughness_bias`), déplacement
centré en relief, albédo centré en luminance 0,5 et désaturé (`detail_saturation`) puis réduit
à 256². Taille physique (`scan_m` = `tile_m` = `GA1_TILE_SIZE`) et raisonnement d'échelle dans
`data/art/materials.yaml`. Chaîne : `uv run --project tools cent-ans assets materials --out
<dossier> --scans --build` (les couches générées sont recopiées des tableaux existants).
