# ADR 0032 — Horizon des batailles : relief réel lointain et panoramas peints

Date : 2026-09-25. Statut : accepté. Lot EP2 du chantier « batailles épiques » (suivi :
`docs/wip/ep2-horizon.md`). Complète les ADR 0017 (atmosphère) et 0020 (relief des champs).

## Contexte

Le joueur veut un horizon de bataille dessiné par la mer, les montagnes, les forêts et les
collines, hors de la zone jouable. Le champ avait déjà un anneau proche (pas de 20 m) et un anneau
lointain (±7 km, pas de 200 m) façonnés par du bruit selon le biome, sous un ciel HDRI : rien de
réel, et rien au-delà de 7 km. Choix validé avec le joueur : **hybride** (relief réel en maillage
lointain + panorama peint par IA pour le très lointain).

## Options

- **Relief réel seul** (DEM jusqu'à 100 km) : maillage énorme ou très grossier, plan lointain à
  16 km de toute façon ; rien pour « habiller » la brume.
- **Panorama peint seul** : bon marché, mais générique, sans lien avec le lieu (des Pyrénées
  peintes au nord d'une bataille de Béarn).
- **Hybride** (retenu) : relief réel jusqu'à 13 km, panorama peint au-delà dont la **ligne de crête
  est recalée sur la ligne d'horizon réelle** calculée hors ligne jusqu'à 150 km.

## Décision

- **Cuisson hors ligne** (`cent-ans geo horizon`, `tools/cent_ans_tools/geo/horizon.py`) : une
  tuile par province (132), 26 × 26 km au pas de 100 m, centrée sur le centroïde (province
  intérieure) ou sur le point de terre le plus proche situé à 3-5 km de la **mer ouverte**
  (province côtière). Altitudes Copernicus GLO-90 moyennées (cache du lot R1), repli sur les tuiles
  fines 360 m hors emprise ; classe par cellule (part de forêt de `splat.png`, ou mer) ; profil de
  ligne d'horizon réel (256 azimuts, 12 → 150 km, courbure terrestre et réfraction standard
  k = 0,13, part de mer). Format binaire dégonflé `HZR1` (≈ 85 ko par tuile, 11 Mo en tout) dans
  `game/assets/horizon/relief/`, index JSON.
- **Raccord sans couture** : `BattleTerrain.world_height` mêle le relief généré et le relief réel
  (recalé : altitude réelle moyenne de l'emprise du champ = hauteur moyenne du champ) entre 0,7 et
  3 km du bord du champ ; la rivière (vallée prolongée, 450 m) et la côte B5 gardent la main. Pour
  une bataille côtière, la tuile est **tournée** pour que la mer réelle tombe du côté du flanc
  côtier tiré par la simulation, et la mer réelle est au niveau de la mer du champ. Les bois
  lointains suivent les forêts réelles. Le champ, la caméra (bornée) et la simulation ne changent
  pas ; rien de tout cela n'a de collision.
- **Anneau d'horizon** (7 → 13 km, pas de 400 m, shader `battle_horizon_land`) : champs, forêts
  réelles, roche, neige au-dessus d'une limite de saison en altitude réelle ; **mer lointaine** là
  où le relief réel est sous la mer.
- **Panorama** : bande peinte sur un cylindre de 14,6 km qui suit la caméra (le très lointain n'a
  pas de parallaxe). L'élévation d'un fragment est mesurée depuis l'œil de référence du profil ;
  chaque colonne est étirée pour que la crête moyenne peinte tombe sur la crête réelle de l'azimut
  (exagérée × 1,35, bornée entre 0,55° et 7,5°) ; secteurs à forte part de mer : horizon marin plat.
  Bande répétée en aller-retour (pas de raccord), décalage tiré de la province.
- **Bibliothèque de 13 panoramas** (Manche, Pyrénées, Alpes, Massif central, collines boisées,
  Picardie, bocage, Loire, Gascogne, Flandre, landes britanniques, Méditerranée, hiver), générés par
  OpenRouter (`openai/gpt-5-image-mini`, 0,57 $ au total, sonde comprise) sur un ciel uni demandé
  dans l'invite, **détourés hors ligne** (modèle de ciel par rangée, crête par colonne, alpha) :
  `game/assets/horizon/panoramas/*.webp` + profil de crête peint (`panoramas.json`). Originaux
  gardés dans `tools/horizon_raw/` pour retoucher le détourage sans nouvel appel payant.
- **Choix du panorama par données** (`data/fx/horizon.json`, schéma `fx_horizon.schema.json`) :
  règles ordonnées (provinces, saison, terrain, côte, hauteur de la crête réelle), la dernière
  attrape tout.
- **Perspective atmosphérique** : quand l'horizon est actif, la densité du brouillard de la scène
  vient de `horizon.json` (temps clair : 0,0002 au lieu de 0,00032, visibilité ≈ 20 km) ; l'anneau
  d'horizon écrit sa propre brume (`FOG`) : même densité sur les 6 premiers km, puis allégée selon
  l'altitude moyenne du trajet (les sommets lointains se lisent) ; le panorama se fond dans la
  couleur du brouillard selon la météo et prend la teinte du soleil (heure, météo).
- **Silhouettes** : 2 à 5 villages (église, clocher, maisons), 0 à 1 château sur une hauteur,
  fumées de village (pas sous la pluie ni la neige), posés entre 2,6 et 8,5 km, fondus par le
  brouillard de la scène.
- `--no-horizon` coupe tout (mesures A/B, repli identique à l'avant-EP2), `--horizon-province=` et
  `--panorama=` servent aux captures.

## Conséquences

- Dépôt : + ≈ 11 Mo de tuiles, ≈ 2 Mo de bandes WebP, ≈ 2 Mo d'originaux JPEG.
- Coût GPU mesuré (voir `docs/wip/ep2-horizon.md`) : quelques milliers de triangles et un cylindre
  plein écran au pire (fragment simple).
- Le relief réel est celui d'un point représentatif de la province, pas du lieu exact de la
  bataille (la campagne n'exporte pas encore de position de bataille) ; les cartes historiques
  (EP7) pourront cuire une tuile sur le site exact avec le même outil.
- Licences : Copernicus DEM (attribution déjà au crédit, lot R1) ; panoramas générés (usage libre).
