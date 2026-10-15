# bm-models : modèles 3D de la carte de campagne (client Bevy)

Branche `bm/models`, dossiers `client/src/campaign/{armies,markers,life}/`.

## Diagnostic (15/10)
- Lieux (cause principale) : Godot affiche sous `near_distance` (1300, au-delà de toute portée de maquette)
  les glb générés DN camp-bati (`data/art/dn_campaign_models.json`) ; Bevy ne montrait que les maquettes du
  kit, dont la famille ouest (matériau `Building` = atlas `building_atlas.gdshader` non porté) sort blanche
  (couleur 0,8 × couleur de sommet, couche d'atlas de l'alpha ignorée). Corrigé : `markers/dn.rs`, 392/392 lieux
  servis (abbaye de Saint-Denis texturée au lieu d'un cube blanc).
- Armées, convois, oiseaux, bateaux : présents (dumps : 13 armées / 40 figurines squelettées, 4 charrettes
  attelées, 76 oiseaux, bateaux de fleuve) ; chargement glb sans erreur.
- Ce qui les « efface » à l'écran, hors de mes dossiers :
  1. brume de l'atmosphère (`campaign/weather/atmosphere.rs`, `fog_strength` 0,54-0,58, densité 0,0025 à d=200) :
     à d=600, avec `CENT_ANS_WEATHER=clear` (fog_strength 0) les figurines redeviennent nettes ;
  2. trous du terrain (tuiles grises/vides à d=25-60 autour de Paris : `campaign/terrain`) ; une charrette posée
     dans un trou paraît flotter ;
  3. bâtiments hors les murs blancs (`campaign/towns/outbuildings.rs`, même matériau `Building`) : lot mats ;
  4. à d<45, 391/392 maquettes enfoncées sous les villes 1:1 / lointain (`towns`), qui ne se voient pas quand le
     terrain manque.

## Restes (mes dossiers)
- Matériau atlas `Building` pour les maquettes du kit (repli seulement désormais) : à partager avec le lot mats.
- Onde de vent des bannières (`maquette_banner.gdshader`), fondu croisé maquette/glb (inutile : near 1300).
- Variante tirée par FNV au lieu de `String.hash` (listes à une entrée presque partout).
