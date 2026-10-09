# QW-E — marche des archers et attributions de figures

## Fait
- `game/scripts/battle/battle_skinned.gd` (`STYLES`) : `bow` et `crossbow` utilisent `bow_walk` / `xbow_walk` pour `marching`, avec repli sur `walk` (kit sans le clip). Les clips sont bien dans `game/assets/models/battle_fine/manifest.json` ; la cadence (1,35) était déjà dans `data/fx/battle_gore.json`. Aucun test existant ne couvre `STYLES`.
- `figure` corrigées (figures existantes, aucun modèle créé) :
  - `unit_mounted_archers` : `cavalry_3` (gendarme à lance) -> `cavalry_2` (style `horse_bow`, arc).
  - `unit_akinci` : `cavalry_6` (lance) -> `cavalry_2` (`horse_bow`), la fiche donne arc court. Leur stat `ranged` reste 0 : ils tirent donc pas en jeu, la figure n'est qu'un rendu ; à rééquilibrer si on veut qu'ils tirent.
- Partage : `cavalry_2` sert désormais mamelouks, steppe, archers montés, akinci (distinction par livrées `unit_looks.json`).

## Restes sans bonne figure
- `unit_yaya` reste sur `archer_3` (francs-archers) : aucune figure d'infanterie orientale à arc n'existe (`infantry_*` sont épée/pique/milice, sans tir). Nécessite une nouvelle figurine (audit 05, ligne 3).

## QW-E suite — figurine yaya (09/10, session de reprise)
Etat : chaine gratuite en cours (cout 0). Plan : feuille Qwen-Image-Edit local depuis la feuille `longbowman`
(`~/dev/cent-ans-raw/ga3/l5/yaya/`) -> decoupe rembg -> TRELLIS HF multivue (ou SF3D de face) ->
`ga3_figures.py` (nouvelle unite `yaya`, figure `archer_6`, copie de `archer_3` dans FIGURES et le manifeste fin)
-> `unit_yaya.figure = archer_6`.
Prochaine etape : verifier `l5/yaya/sheet.png`, puis 3D.

Etat 09/10 (arret) : placeholder `archer_6` (FIGURES + manifeste fin) et unite `yaya` de `ga3_figures.py` commites ;
`unit_yaya.figure` reste `archer_3`. La generation Qwen locale (feuille `~/dev/cent-ans-raw/ga3/l5/yaya/`, prompt.txt pret)
a ete interrompue a 60 % sur consigne du coordinateur (passage a fal : Z-Image Turbo + TRELLIS 1, enveloppe 1 $ de
`docs/budget.md`). L'appel `dn_batch.py` fal (catalogue `fig_yaya`, `DN_FAL_CAP_USD`=44.512, `--until 3d`) a ete refuse par
le systeme de permissions (depense cloud sans autorisation directe du joueur) : aucun appel fait, cout 0.
Reprise : soit autoriser la depense fal, soit relancer la commande Qwen locale (~45 min) puis TRELLIS HF / SF3D.

## QW-E : TERMINE (09/10)
- Image/3D faites par le coordinateur (fal, 0,049 $, `docs/budget.md`) : `~/dev/cent-ans-raw/dn/yaya_archer_6/` (face `img/s1337.png`, glb `3d/fal__s1337.glb`). Copies dans `~/dev/cent-ans-raw/ga3/l5/yaya/` (`multi_front_back.glb`, `sheet.png`).
- Bake : `blender -b --factory-startup --python tools/blender_scripts/ga3_figures.py -- yaya` -> `battle_ga3/archer_6_*` (LOD 11633/1310/260 tris, 1 tete, pas de variante de visage).
- `archer_6` : maillages fins copies d'`archer_3` (`battle_fine/archer_6_lod*.mesh.bin`, necessaires au bake pour les mains) ; `unit_yaya.figure = archer_6`.
- Tests : `ga3_l3_figures_test.gd` OK (archer_6 ajoute, sans buste en livree : cuirasse cloutee), `test_ga3_figures_manifest.py` 4 OK. `smoke.gd` : echecs sans rapport (erreurs de compilation `parchment_overlay.gd`/`strategic_view.gd` de l'arbre de travail d'autres agents).
- Verif visuelle : rendu Blender (bind/dos/marche/tir), pas de capture Godot. Defaut : en tir, la jupe rouge (livree) s'etire en lamelles ; pas d'armoiries sur la poitrine.
