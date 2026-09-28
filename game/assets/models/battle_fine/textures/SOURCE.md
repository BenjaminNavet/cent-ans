# Textures des figurines fines — provenance

- `fine_atlas_lod0.png`, `fine_atlas_lod1.png`, `fine_detail.png` : cuits par le projet (lot FG3,
  ADR 0088 ; `tools/blender_scripts/battle_fine_bake.py`, `battle_fine_tiles.py`).
  `fine_detail.png` n'est chargé qu'avec `--no-ga1`.
- `fine_horse.png` : pelage de cheval CC0 réduit (ADR 0088).
- `fine_detail_ga1.png` (12 couches 512² : RG normale, B relief, A rugosité) et
  `fine_detail_albedo.png` (12 couches 256², albédo de détail centré) : **générés, projet**
  (lot GA1, ADR 0104). Images sources `openai/gpt-5-image-mini` via OpenRouter, prompts dans
  `data/art/materials.yaml` (ordre = couches), chaîne `tools/cent_ans_tools/material_gen.py`
  (`cent-ans assets materials`, puis `material_gen.build_fine_arrays(<dossier>)`). Images
  brutes non versionnées. Coût consigné dans `docs/budget.md` (section GA).
