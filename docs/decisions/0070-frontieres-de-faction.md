# ADR 0070 — Frontières de faction lumineuses peintes dans le fragment du terrain (lot FR1)

Date : 2026-09-26. Statut : accepté.

## Contexte

La carte de campagne dessinait ses frontières dans `terrain.gdshader` : trait noir fin entre
provinces, trait noir plus épais et liseré sombre de la couleur de faction aux frontières de
royaume. Peu lisible au dézoom, rien pour l'occupation, rien de propre au joueur. On veut des
frontières « façon Total War » : trait de la couleur de chaque faction, halo doux à l'intérieur
aux frontières nationales, trait fin et discret entre provinces d'une même faction, frontière du
joueur rehaussée, provinces occupées (contrôleur ≠ propriétaire) hachurées ; drapées sur le relief
à tous les zooms (quadtree ZG2 jusqu'à 1-5 m), effacées sous ~200 m, plus appuyées en vue
parchemin (CM2), respectant les filtres de carte (MF1), mises à jour dès qu'une province change de
mains ; budget ≤ 0,5 ms GPU à 1080p en Haute.

Données disponibles sans rien ajouter au cœur : raster des provinces (`province_ids.png`), champ de
distance signé aux frontières terrestres (`province_border_dist.png`, zéro sous-pixel entre deux
provinces), propriétaire et contrôleur par province (`get_province_state` du pont).

## Options essayées

1. **Rubans maillés** (polylignes extraites du raster, `tools/`, maillage drapé reconstruit à chaque
   changement). Écarté : la hauteur affichée dépend du nœud du quadtree (morphing CDLOD, exagération
   ZG4/ZG8 variables) ; un ruban flotte ou s'enfonce selon le LOD, surtout sur les crêtes, sauf à
   rejouer tout le quadtree côté ruban.
2. **Passe suivante (`next_pass`) du matériau du terrain**, shader séparé rejouant le chemin de
   sommets du terrain (`qt_vertex`, lit creusé, mêmes uniformes d'instance). Implémenté et mesuré :
   drapé parfait, aucune ligne de `terrain.gdshader` touchée, mais **+1,1 ms (Europe), +4,1 ms
   (comté), +1,0 ms (vallée)** de GPU : tout le relief est redessiné (sommets du quadtree et tous
   ses pixels, même quand ils sortent au premier test). Hors budget.
3. **Décalque plein écran** (profondeur reconstruite). Écarté : peindrait aussi arbres, villes et
   armées (rien ne distingue le sol dans le tampon de profondeur).
4. **Crochet dans le fragment du terrain** (retenu).

## Décision

- `game/shaders/faction_borders.gdshaderinc` : uniformes `fr1_*` et fonction
  `fr1_borders(p, uv, footprint, albedo, emission)`. Inclus par `terrain.gdshader` et
  `terrain_parchment.gdshader` : **deux lignes par shader** (l'include, l'appel juste après
  `parchment_apply` / `parchment_only`), sur le modèle des crochets ZG2 (`qt_vertex`) et CM2
  (`parchment_apply`). Aucune autre ligne des shaders du terrain ne change.
- La fonction relit le champ de distance, détermine les deux côtés du trait par son gradient (même
  méthode que le terrain et le parchemin), lit propriétaire / contrôleur / drapeau joueur des deux
  provinces (texture 1D `fr1_info`) et la couleur de faction (palette `fr1_palette`), puis compose :
  halo intérieur (frontière de royaume, couleur de CE côté), hachures diagonales de l'occupant
  (province occupée), liseré sombre, trait coloré. Largeurs en pixels écran le long de la normale
  au trait (nettes en vue rasante) ; de près, distance en bilinéaire manuel (le filtrage matériel
  n'a que 8 bits de poids : trait pointillé au zoom vallée). Couleur en albédo + part émissive
  (lisible à l'ombre du relief) ; en vue parchemin, partage albédo / émission du papier
  (`pm_output`), traits plus larges, halo réduit, teinte d'encre.
- `fr1_alpha` vaut 0 par défaut : tout autre utilisateur de ces shaders (échantillons de légende…)
  n'a aucun effet et ne paie qu'un test uniforme ; le coût réel se limite aux pixels proches d'une
  frontière.
- `FactionBorders` (`game/scripts/map/faction_borders.gd`) pose les uniformes sur le matériau
  partagé du terrain : textures reconstruites à `refresh_all` seulement si propriétaires ou
  contrôleurs ont changé (changement visible à l'image suivante, aucune géométrie), opacité =
  filtre de carte × fondu de zoom (200 → 1 400 m de distance caméra), largeur accrue au dézoom,
  respiration lente du halo du joueur, qualité PF1 (Basse : sans halo ni hachures). Le liseré
  sombre intérieur du terrain (`realm_band_alpha`) est coupé : le halo le remplace.
- Filtres de carte (MF1) : politique pleine opacité ; diplomatie et revendications en couleurs de
  faction atténuées ; religion, mécontentement, richesse, population, loyauté à l'encre neutre
  (les couleurs de faction se confondraient avec celles du filtre) ; ravitaillement masquées.
- Réglages dans `data/map/faction_borders.json` (schéma `faction_borders.schema.json`, pytest).
  Option `--no-faction-borders` (A/B).

## Conséquences

- Coût mesuré (`game/tests/fr1_shot.gd`, 1080p, Haute, Vulkan, A/B entrelacé, médianes) : de
  l'ordre de 0,1 à 0,5 ms selon le zoom (Europe le plus cher : beaucoup de pixels de halo), dans le
  bruit de mesure (±0,3 ms) ; l'état « éteint » garde une lecture bilinéaire et deux dérivées par
  pixel (négligeable).
- `terrain.gdshader` (lot ZG) et `terrain_parchment.gdshader` (lot CM2) portent chacun deux lignes
  FR1 ; toute refonte de ces shaders doit garder l'appel `fr1_borders` en fin de fragment (après
  la couleur finale, avant `ALBEDO`), le test `fr1_borders_test.gd` le vérifie.
- Les frontières restent celles du raster des provinces : pas de frontière maritime ni de
  frontière dans les lacs (le champ de distance ne couvre que les frontières terrestres).
- Captures : `docs/audit/captures/fr1/`.
