# Pipeline / tools developer — état des lieux (09/10, lecture seule)

## 1. État actuel
- Paquet `tools/pyproject.toml` (`cent-ans-tools`, uv, Py 3.12), CLI typer `cent-ans` (~68 commandes, dont 28 geo_map, 17 assets_media), 39 modules.
- 190 fichiers de tests (`tools/tests/`), surtout schémas (~213 dans `data/schemas/`) ; pipeline 3D : `test_dn_batch_*`, `test_dn_ingest`, `test_ga3_*`.
- Pipeline image→3D (ADR 0140, 0210-0212, 0214, 0221, 0222) : Z-Image/Qwen local (mflux) → rembg → TRELLIS (fal / HF) → SF3D repli → `cent-ans dn-ingest` (Blender) → `dn_manifest.json` → paquet `models-v<N>`.
- `tools/experiments/dn_batch.py` (1 235 l.) : reprenable, `--until select`, contrôle charte D5, repli local, journaux jsonl, verrou `tools/gpu_lock.sh`.
- Galeries : `dn_live_gallery.py` (port 8765) ; `~/dev/cent-ans-raw/galerie-3d/build.py` **hors dépôt**.
- 57 scripts Blender (`tools/blender_scripts/`) + 2 (`tools/blender/`).
- Captures : 15 `game/tests/*_shot.gd`, `tools/godot_bg.sh`, `tools/godot_shot.sh` + Dockerfile (jamais exécuté jusqu'au bout).
- `tools/proto_moteur/` ni suivi ni ignoré.

## 2. Forces
- Provenance : `generation.json` par objet, seeds déterministes, prompts versionnés.
- Budget intégré (`budget.check`, `BudgetExceeded`).
- `dn-ingest` idempotent, budgets tris, plafond saturation.
- Recuisson `ga3_figures.py` identique à l'octet.
- Doc de procédé riche (`docs/wip/i3d-local.md`), verrou GPU, manifestes sous schéma.

## 3. Faiblesses
1. Pas de CI pytest (seul `windows.yml`).
2. Outils de production dans `experiments/`, deps non verrouillées (`uv run --with …`), tests via `sys.path.insert`.
3. Galerie 3D hors dépôt, aucun test de `dn_live_gallery`.
4. Captures fragiles : `godot_bg.sh` sans code de sortie et vol de focus ~1,7 s ; `godot_shot.sh` non validé ; amorçage dupliqué dans les 15 `*_shot.gd` ; pas de diff d'images.
5. Reproductibilité : chemins de poids en dur sans empreinte/versions ; patchs SF3D manuels ; `trellis_hf.py` dupliqué.
6. Charte D5 mal calibrée : p95 S rejette tout le bois (0,83-0,86 vs 0,40) → contournement `--charter-s-p95 0.9`.
7. `fal_spend.jsonl` reporté à la main dans `budget.md`.
8. `docs/tools.md` obsolète (~9 commandes sur 60+).
9. Pas de schéma pour `dn_catalog_*.json`.
10. Scripts Blender dispersés, vivants/morts non distingués.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| 1 | Workflow `tools.yml` : uv sync + ruff + pytest | Fort | S | 0 | build |
| 2 | Schéma `art_dn_catalog` + test | Fort | S | 0 | DA |
| 3 | Rapatrier `galerie-3d/build.py`, statuer sur `proto_moteur` | Moyen | S | 0 | producer |
| 4 | Promouvoir `dn_batch`/galerie en sous-commandes, extras `[gen]` | Fort | M | 0 | — |
| 5 | Provenance complète (sha256 poids, versions outils, commit) | Fort | S | 0 | — |
| 6 | Recalibrer D5 (p95 par matière) + bible § 14.2 | Fort | S-M | 0 | DA, ADR |
| 7 | `budget import-fal` automatique | Moyen | S | 0 | producer |
| 8 | Finir CF : valider `godot_shot.sh`, ADR | Fort | M | 15-20 Gio disque | tech, joueur |
| 9 | Helper `tests/shot_util.gd` + non-régression visuelle | Moyen-fort | M | 0 | rendu |
| 10 | Script d'install reproductible (SF3D, poids mflux, `doctor`) | Moyen | M | 0 | — |
| 11 | Régénérer `docs/tools.md` | Moyen | S | 0 | — |
| 12 | Tests de fumée galerie + Blender (fixture glb) | Moyen | M | 0 | — |
| 13 | Fusionner les dossiers Blender, inventaire actif/archivé | Faible-moyen | M | 0 | 3D |

Ordre conseillé : 1, 2, 3, 5, 11 (≈ 1 jour) → 4, 6, 7 → 8, 9.
