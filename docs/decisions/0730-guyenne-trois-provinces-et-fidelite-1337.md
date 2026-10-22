# 0730 — Guyenne en trois provinces, Lothian écossais, Pays de Galles, champs de bataille historiques variés

Statut : accepté (22/10, lot B3 des critiques de nuit, `docs/wip/nuit-critiques/level-design.md` constats 1, 3, 4, 6, 12).

## Contexte
La critique de level design relevait : toute la position anglaise tenait en une province (`prov_guyenne`, Bordeaux + Landes +
Bayonne) ; la province « Galles du Nord (Gwynedd) » avait Carmarthen pour cité ; l'Écosse ne possédait que Fife alors que sa
capitale de faction est Lothian (tenue par l'Angleterre) ; seuls Crécy, Poitiers et Azincourt, tous en terrain `plains`,
étaient jouables en bataille historique ; `neighbors` était vide dans 208 fichiers de provinces.

## Décision
1. **Guyenne en trois provinces anglaises** : `prov_guyenne` (Bordelais : Médoc, Entre-deux-Mers, Libournais jusqu'à Castillon ;
   Bordeaux, Libourne), `prov_bazadais` (Bazas, La Réole ; front de la Garonne face à l'Agenais), `prov_gascogne` (Landes,
   Marsan, Labourd ; Bayonne, Dax). La géométrie reste celle des provinces d'origine de la carte composée ; la scission
   se fait dans `compose/provinces_composed.py` par graine la plus proche en pixels réels (`data/map/compose/province_splits.json`,
   poids ajustable). Les populations conservent leur total (70 % / 30 % de l'ancienne Guyenne, la Gascogne intacte), le
   `base_supply` (5 000) est réparti au prorata de la population (2 300 / 1 000 / 1 700). Périgord reste dans la province
   française d'Agenais (Périgord, comté français en 1337) : ni la critique ni l'histoire ne l'attribuent à l'Angleterre.
2. **Pays de Galles** : l'id `prov_gwynedd` est gardé (nombreuses références), le nom affiché devient « Pays de Galles » (la province
   regroupe Gwynedd et Deheubarth). Carmarthen reste la cité : siège de l'Échiquier du Sud, Caernarfon étant le siège du Nord
   sans lieu propre dans les 400 lieux composés.
3. **Lothian** : `fac_scotland` en 1337. David II est en exil mais ses gardiens (Stewart, Murray) tiennent le pays ; l'Angleterre ne
   garde que des garnisons (château d'Édimbourg 1335-1341). L'Angleterre en conserve la revendication (`claims`) ; l'événement
   historique de reprise d'Édimbourg ne transfère plus la province (prestige et opinion seulement).
4. **Champs de bataille** : Auray (1364, `heath`), Formigny (1450, `bocage`), Castillon (1453, `marsh`) rejoignent Crécy, Poitiers
   et Azincourt (laissés en `plains`, relief cuit réel). Sans `relief` cuit (pas de DEM hors ligne) : le relief vient du profil de terrain.
   Chaque carte porte une fenêtre `campaign` limitée à la période où la bataille a un sens.
5. **`neighbors`** des fichiers de provinces : dérivé de l'adjacence terrestre de `provinces.geojson` (calculée sur la géométrie), test de
   cohérence.

## Conséquences
- 215 provinces (213 + 2). Les tests Rust qui supposent « England = 10 provinces » ou l'adjacence de Guyenne sont recalés.
- La chaîne géométrique (`geo provinces` … `cross_fixture`, `docs/geo.md` § lot CC C4b) a été relancée.
- Les cartes d'Auray, Formigny et Castillon n'ont pas de relief DEM : à cuire avec `cent-ans geo battle-site` en ligne.
- Un joueur anglais a une défense en profondeur (trois cibles : Bordeaux, Bazas, Bayonne).
