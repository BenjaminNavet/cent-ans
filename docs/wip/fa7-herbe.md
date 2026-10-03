# FA7 — herbe des batailles en vrais brins (2026-10-02)

Lot du chantier FA (`docs/wip/fa.md`). Branche `feat/fa-grass`, worktree `../gp-fa-grass`.
Brutes et captures hors dépôt : `~/dev/cent-ans-raw/fa/ambientcg/`, `~/dev/cent-ans-raw/fa/grass-shots/`
(`shots.sh <tag> <core|all|vues> [args]` prend les vues, `bench.sh` lance le banc A/B entrelacé,
`setp.py clé=valeur` règle le catalogue, `plan_*.json` posent une grande parcelle de blé, de chaume
ou de semis).

## Consigne de reprise
> Lis ce fichier et `git log --oneline -10`. Le lot est fait ; reste la fusion dans `feat/fa`
> (lot FA4 : ADR du chantier, à compléter avec « Décisions » ci-dessous).

## Fait
- Atlas `game/assets/textures/battle/grass_tufts.png` (2048 × 768, 4 × 3 cases de 512 × 256, BC7,
  mipmaps, ~2 Mo) : 12 touffes (3 rases, 3 hautes, 2 à épis, 2 herbes folles avec pissenlit et
  pâquerettes, 2 de blé) composées par `build_fa_grass.py` depuis Foliage001-008 et LeafSet020
  (ambientCG, CC0) : brins détourés, mis à l'échelle en mètres, penchés, courbés, posés sur
  plusieurs pieds resserrés.
- Catalogue `data/art/battle_grass.json` (sources, touffes, `render`), schéma
  `data/schemas/art_battle_grass.schema.json`, `tools/tests/test_art_battle_grass_schema.py`
  (schéma, références, grille, marges, luminance de l'atlas), `game/tests/fa7_grass_test.gd`.
- Shader `battle_grass.gdshader`, chemin `fa_on` (l'ancien chemin est intact) : case tirée par
  carte (hachage de la cellule monde, poids selon le bouquet), carte retournée une fois sur deux,
  hauteur de la case, herbe rase dans les vides (densité 0,62 → 1,0 entre les bouquets, trouées à
  25 m gardées à 0,85), clarté ×1,25 et normale verticale (éclairage du sol), pied à 0,95 au lieu
  de 0,8, écarts de teinte entre touffes réduits de moitié, herbe couchée à 16 cm (BV3).
- `BattleVegetation` : maillage de 2 cartes croisées d'un mètre (au lieu de 4), larges de 0,8 m,
  pied à 80 % ; `--no-fa-grass` après `--` rend l'herbe d'avant ; `--bench-ab=fa-grass,no-fa-grass`
  bascule les deux herbes dans le même processus.
- Parcelles : blé = cases `wheat_a/b` (épis barbus serrés, teinte dorée, pied ombré), chaume et
  semis = herbe rase (semis reverdis).

## Décisions (pour l'ADR du chantier, lot FA4)
- **Couleur** : carte neutre en moyenne (`neutral_pull` 0,85), luminance linéaire moyenne
  `render.tex_lum` ; le shader multiplie la couleur du sol par l'écart de clarté et de teinte du
  brin (`hue_mix` 0,35). La couleur réelle n'a pas été essayée : les saisons, la neige, le sang
  et les parcelles règlent déjà la teinte par le sol, et le premier rendu fondait bien. Les épis
  sortent paille, les brins verts.
- **Deux cartes par touffe** : trois cartes coûtaient +8 à +10 % de temps d'image en vue
  rapprochée, deux cartes −5 à −9 % par rapport à l'herbe d'avant (4 cartes, moins de touffes
  vivantes), sans différence visible dans une nappe continue.
- **Mesure** : le banc à 1280 × 720 colle au palier de 16,67 ms ; mesurer avec
  `--disable-vsync`, 1920 × 1080 et le banc A/B entrelacé (machine chargée par d'autres agents).

## Jugement sur captures (planches `~/dev/cent-ans-raw/fa/grass-shots/planche_*.jpg`)
- Vue rapprochée (forêt, plaine, collines, marais) : nappe de brins au lieu de plants isolés ;
  net mieux. En forêt, le sol clair du sous-bois perce par plaques à bord net.
- Vue moyenne (mêlée) : équivalent, moucheté sombre en moins.
- Vue haute (déploiement) : identique (herbe masquée au-dessus de 170 m).
- Automne, hiver sans neige : mieux ; neige : identique (herbe presque toute retirée).
- Blé : champ d'épis serré au lieu de touffes ; chaume : paille rase ; semis : pousses peu
  visibles (moins lisibles qu'avant de loin).
- Herbe foulée : brins couchés lisibles, zone un peu plus nue qu'avant. Sang sur l'herbe : non
  jugé (la capture `bv3_shot --shot=grass` ne cadre aucun mort) ; le code de teinte est inchangé.

## Points ouverts
- Fleurs dessinées par le script (pas d'atlas de fleurs CC0 dans les sources) et peu visibles :
  la teinte du sol les ternit.
- Semis moins lisibles qu'avant (pousses vertes discrètes sur la terre).
- `docs/wip/fa.md` et l'ADR du chantier sont à compléter par la session principale.
