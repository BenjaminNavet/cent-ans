# DN fig-bake : chaîne de cuisson des figurines de bataille

Branche `dn/fig-bake`. État : **terminé**, non fusionné.

## Fait
- Restaurés depuis `a600b88f2^` : `tools/blender_scripts/ga3_figures.py` (+ variable `GA3_OUT_DIR`
  pour rediriger la sortie), `tools/blender_scripts/ga3_figure_probe.py` (importé par le précédent),
  `tools/experiments/ga3_fal_figure.py` (prompts de référence), `tools/tests/test_ga3_figures_manifest.py`.
  Les autres fichiers du chantier SC (`ga3_cleanup`, `ga3_sheet`, `ga3_siege_rig`, végétation, décor)
  ne concernent pas les figurines et ne sont pas restaurés.
- Recuisson de `man_at_arms` (infantry_0) dans un dossier temporaire : LOD0/1/2 `.mesh.bin`,
  albédo et entrée de manifeste **identiques octet pour octet** aux fichiers commités. Aucun écart.
  Effet de bord : Blender réécrit `~/dev/cent-ans-raw/ga3/l3/man_at_arms/ga3_man_at_arms.blend`.
- Commande documentée : `docs/pipeline-assets-3d.md` § 6 « Figurines de bataille : glb → jeu ».

## Types d'unité sans champ `figure` (13) et variante GA3 la plus proche (données non modifiées)
| Type | Famille | Proposition |
|---|---|---|
| longbowmen | archer | archer_0 (longbowman) |
| crossbowmen | archer | archer_2 (crossbowman) |
| genoese_crossbowmen | archer | archer_1 (plain_crossbowman) ; pavois à ajouter côté équipement |
| men_at_arms_foot | infantry | infantry_0 (man_at_arms) |
| flemish_pikemen | infantry | infantry_4 (schiltron) ; à défaut infantry_5 (militia, goedendag) |
| urban_militia | infantry | infantry_2 (urban_militia) |
| knights | cavalry | cavalry_0 (knight) |
| mounted_sergeants | cavalry | cavalry_3 (gendarme), seul autre cavalier GA3 ; cavalry_4 fin serait plus léger |
| mounted_archers | cavalry | aucun cavalier-archer GA3 : cavalry_3 par défaut, ou cavalry_2 (fin, steppe) plus juste |
| bombard, trebuchet, mangonel, siege_tower | siege | pas de figurine : engins (`models/siege/*.glb`), servants `crew_0/1` fins sans GA3 |

Point ouvert : remplir ces champs est une décision de données (`data/unit_types`), non faite ici.
