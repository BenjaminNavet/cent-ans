# V2 — Terrain de campagne semi-réaliste (branche `visual-v2`)

Plan : `docs/design/visuel-semi-realiste.md` (lot V2), ADR 0004. **État : terminé**, à fusionner par
l'orchestrateur. Captures : `docs/img/visuel/v2_*.png` (`v2_campaign_far`/`v2_campaign_near` = mêmes
cadrages que les `before_*` ; `v2_map_far`, `v2_map_mid`, `v2_alps`, `v2_alps_far`, `v2_coast` sans sélection).

## Fait

- **Relief** : exagération verticale de la géométrie ramenée de ×14 à ×4,3 (`MapData.HEIGHT_SCALE`
  0,02 → 0,006, source unique pour terrain, villes, armées, picker, fleuves). Heightmap envoyée au GPU en
  `Image.FORMAT_R16` (octets little-endian du décodeur Rust, filtrage matériel exact) avec mipmaps : fin des
  stries noires des Alpes (l'interpolation séparée des octets LA8 et l'échantillonnage sans mipmaps au
  dézoom). Normale d'ombrage à pas proportionnel à l'empreinte du pixel (`shading_relief` ×1,8 par-dessus
  la géométrie). LOD lointain lissé (bilinéaire dans le mipmap 4×4).
- **Rasters hors ligne** (`uv run --project tools cent-ans geo splat`, `tools/cent_ans_tools/geo/splat.py`,
  tests `tools/tests/test_splat.py`, ~50 s) :
  - `data/map/splat.png` (RGBA8 2048²) ;
  - `data/map/province_border_dist.png` (RGB8 4096²) : 3 champs de distance **signés**, signe = un bit
    d'une coloration du graphe des provinces (≤ 8 couleurs) → `min(|R|,|G|,|B|)` filtré bilinéairement
    passe par zéro exactement sur la frontière (trait sous-pixel, sans escalier) ;
  - `data/map/coast_dist.png` (L8 4096²) : distance signée au rivage (mer = altitude ≤ 0, lacs du masque).
- **Textures PBR CC0** Poly Haven 1k (`cent-ans geo textures`, `tools/cent_ans_tools/geo/textures.py`,
  `game/assets/textures/terrain/README.md`) : 7 couches (prairie, cultures, forêt, roche, lande, neige,
  sable), albédo + (normale XY, rugosité) regroupés au chargement en deux `Texture2DArray`.
- **Shader terrain** (`game/shaders/terrain.gdshader`) : albédos réalistes (teintes linéaires réglables,
  détail des textures normalisé par leur moyenne), tuilage adapté au zoom (deux octaves d'échelle, pas
  de trame répétée), normales de détail combinées au relief, parcellaire procédural (Voronoï allongé,
  orientation par terroir, haies) sur les cultures de près, neige au-dessus de ~2 750 m, roche en
  altitude/pente, sable des plages, lacs peints. Couche politique : teinte de faction à luminance
  constante (10 % de près → 50 % au dézoom + léger aplat/désaturation « carte »), frontières de province
  fines et de royaume marquées (trait plus épais + liseré intérieur de la couleur de faction, détecté
  en comparant la couleur de la province d'en face) ; surbrillance survol/sélection et masque
  d'atteignabilité conservés.
- **Eau** (`water.gdshader`) : couleur et opacité selon la profondeur, houle en normales (estompée au
  dézoom), reflet du soleil par `light()` Blinn-Phong borné (plus de tache blanche), écume animée le long
  des côtes, côte précise via `coast_dist` (l'eau s'efface sur la terre même si le maillage LOD passe
  sous 0 ; le terrain peint la mer là où le maillage dépasse le plan d'eau).
- **Fleuves et côte** : `terrain_line.gdshader` + `PolylineMesh.build_screen_lines` : rubans élargis dans
  le vertex shader à 1,6 px (fleuves majeurs) / 1 px (mineurs) minimum, estompés s'ils sont plus fins,
  couleur accordée à la mer ; trait de côte discret (1 px, 28 %).
- Performance mesurée (`--print-fps --disable-vsync`, 1440×900, autres agents actifs) : ~86 i/s au zoom
  moyen, 120 de près, > 200 au dézoom. Construction du terrain ~430 ms.

## Changements hors périmètre (minimes)

- `game/scripts/map/campaign_map.gd` : option `--stage=map` (capture sans sélection ni panneau).
- `tools/cent_ans_tools/cli.py` : commandes `geo splat` et `geo textures`.
- `Sea` lit la heightmap et `coast_dist` du nœud frère `Terrain` (pas de changement de `campaign_map.gd`).

## Contrat avec V3 (végétation) et les autres lots

- `data/map/splat.png` : RGBA8, R = prairie, G = cultures, B = forêt, A = roche/lande ; poids 0-255
  normalisés (somme = 255 sur terre, 0 sur l'eau), même repère que `province_ids.png`, 2048² (lire la
  taille dans le fichier). `MapData.splat_image` le charge déjà (null si absent).
- Échelle verticale : une seule source, `MapData.HEIGHT_SCALE` (unités monde par mètre, 0,006 ≈ ×4,3) ;
  poser les objets avec `MapData.surface_world_at(x, y)`. Attention : le LOD lointain est lissé (écart
  de quelques dixièmes d'unité sur les sommets).

## Points ouverts

- Réglages fins possibles (tous en uniforms) : teintes, `political_*`, largeurs de frontières.
- Parcellaire assez géométrique au zoom le plus proche ; forêts en volume attendues du lot V3.
- Les ombres portées énormes des marqueurs d'armée (V1/V3) tachent la carte au zoom moyen.
