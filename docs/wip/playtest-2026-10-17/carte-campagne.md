# Rapport : carte de campagne (rendu 3D et monde)

Dossier de travail : `$P/carte-campagne/` (P = .../scratchpad/playtest). Outil : `carte-campagne/run.sh <nom> "<lon,lat,dist>" [ENV=..]`
(lance `play.sh --faction fac_france` avec `CENT_ANS_CAMERA`, attend 500 images, dump + capture). ~45 captures, 3 saisons, 5 météos.
Remarque de méthode : `CENT_ANS_SEASON` ne change que le sol/végétation (le HUD reste « Printemps 1337 »), c'est normal.
Aucun panic ni ERROR dans les journaux ; seul WARN récurrent : `bevy_light: ... no entity has ShadowLodOrigin` (cosmétique).
Dumps : aucun NaN/Inf dans campaign_terrain/water/freshwater/vegetation/towns/markers ; les zéros relevés sont attendus (villes 1:1 inactives au-delà de ~45).

## MAJEURS

### 1. Rochers rendus comme d'énormes blobs noirs « tramés » (Alpes, Pyrénées, Petra, Espagne, toute la montagne)
- Repro : `./run.sh alps50 "7.0,45.9,50"`, `alps150 "7.0,45.9,150"`, `pyr100 "0.5,42.7,100"`, `petra40 "35.44,30.33,40"`.
- Observé : des centaines de masses sombres quasi noires, au motif de bruit (alpha-hash / dithering), grosses comme des maisons, posées sur la neige et la roche : on dirait des trous ou des taches. Aucune lecture de « rocher ». Pire sur neige (alps50.png, crop_alps_rock.png) et sur sol clair (petra40.png). Attendu : rochers gris texturés opaques.
- Dump : `rocks.live=2795`, `scale=2.47` à d=50 (`far_scale` 2.6), `opacity=1.0`, `models_resolved=26`.
- Cause probable : `client/src/campaign/countryside/rocks.rs` (en-tête l.20 : « Not ported: the shader albedo gain and pixel fade ») matériau glTF brut sombre + grossissement ×2,5 à partir de d≈15 (`scale_for`, l.519) ; à vérifier : texture des `models/*_lod*.glb` non liée ou mode alpha.

### 2. Baie du Mont-Saint-Michel : dalle noire plate avec « murs » beiges droits
- Repro : `./run.sh msm40 "-1.51,48.636,40"` (et `msm100`, `coast60 "-1.5,48.64,60"`).
- Observé : une grande baie parfaitement noire (aucun relief ni eau), avec des filets de rivière dessinés dessus, bordée de falaises beiges verticales rectilignes ; une frontière traverse l'eau en ligne droite. Aucun repère du Mont-Saint-Michel visible. Attendu : estran/eau lisible et le landmark.
- Cause probable : lot récent mapgeo11 (« warped inlet taper », commits beb5a2378 / aae3af7db) ; matériau d'eau/estran de l'anse absent ou hauteur du sol écrasée.

### 3. Lac Léman : feuille plate flottante, trou noir triangulaire
- Repro : `./run.sh lake_leman "6.5,46.45,45"` ; aussi visible en `alps150` (parallélogramme sombre incliné).
- Observé : le lac est un plan à hauteur constante qui coupe les versants, avec un grand triangle noir (trou) à gauche, des lignes de frontière tracées sur l'eau, des arbres qui mordent sur le plan. Attendu : lac encaissé, sans trou.
- Preuve : `lake_leman.png`, `alps150.png`.

### 4. Londres : la Tamise est un disque gris à plat, le pont ne franchit rien ; Constantinople déborde dans le Bosphore
- Repro : `london30 "-0.12,51.5,30"`, `constantinople "28.97,41.01,40"`.
- Observé : à Londres, un aplat gris (type béton) entoure la maquette, des barques y reposent, et le pont en croix s'avance sur la terre. À Constantinople, la plate-forme carrée de la ville couvre la largeur du détroit et dépasse sur l'eau côté sud ; côtes en gradins (bandes cyan empilées, rive est). Venise (`venice`) : angle de la plate-forme sur l'eau (mineur, ponton).
- Cause probable : emprise de la maquette non clippée au masque terre/eau (`campaign/towns`, landmarks `landmark_anchored: 8`).

### 5. Rivières : ruban noir, haché, enterré par le relief (Douro, Seine aval, Garonne)
- Repro : `spain250 "-3.7,42.3,250"` (crop_douro.png), `z_250` (Seine en bas à droite).
- Observé : en aval les rivières sont un ruban presque noir interrompu en tirets là où le relief le recouvre (z-clipping), largeur très différente de l'amont (turquoise clair). Attendu : eau continue, de couleur cohérente.
- Cause probable : maillage de rivière non plaqué sur `SurfaceHeight`/relief exagéré (campaign/water).

### 6. Mât gigantesque au-dessus de Saint-Denis
- Repro : `./run.sh z_50 "2.35,48.85,50"` (crop_pole.png), visible aussi en `paris30`, `z_90`, `winter_paris60`, `rain_paris100`.
- Observé : un fin mât brun, sorti du clocher de Saint-Denis, monte hors de l'écran (>100 u de haut). Attendu : fanion à l'échelle.
- Cause probable : modèle de bannière/mât de lieu avec échelle erronée (campaign/markers ou countryside_decor).

## MINEURS

### 7. Aspect « low poly » des forêts et collines lointaines
- `london30.png`, `lake_leman.png`, `msm40.png`, `z_50.png` : canopées en pyramides/cubes anguleux, crêtes lointaines en dents de scie (rome30.png, haut d'image), gros cubes gris en plaine d'Île-de-France. Contredit « pas de low poly ».
- Hiver : les forêts deviennent des nuages gris tramés (snow_london100.png), halftone très visible au sol (spain100, rome100).

### 8. Échelle incohérente de la faune
- Oiseaux de la taille d'un cheval près de la caméra (z_90.png, coast60.png) ; troupeaux de chameaux grands comme la pyramide (giza40.png).

### 9. Pétra
- `petra40.png` : le monument est une grosse masse rose amorphe (modèle fondu/illisible), entourée de rochers noirs (cf. 1) ; textures très étirées sur les pentes. Giza OK mais très proche du Caire (giza40.png) et Le Caire est posé sur un socle ovale qui surplombe le Nil.

### 10. Étiquettes et plaques
- Vue parchemin (`z_far.png`) : plaque d'armée « 620 » recouvre le nom « Paris » ; plaques « 100/240 » qui se chevauchent.
- `pyr100.png` : « Toulouse » décalée d'~110 px de la maquette ; étiquettes de rivière répétées (Marne x4, Meuse x4).
- Roma (`rome30`) : nom « Rome » masqué par la maquette.

### 11. Météo forcée peu lisible
- `CENT_ANS_WEATHER=fog` (fog_paris60) : brume 75→330 contre 112→495 en temps clair (dump `campaign_weather.atmosphere.fog`) : aucun brouillard perceptible. `storm` à d=600 identique au clair (pas de pluie sous 220, attendu, mais pas d'assombrissement non plus).
- Vue 400 (`z_400.png`) : sol flou, taches vertes répétitives (tuilage perceptible).

### 12. Littoral crénelé
- `med400.png` (Malte/Mdina) : côte en escalier pixélisée ; `rome100.png` : écume en festons réguliers.

## Ce qui marche bien
- Paliers de zoom 3000 -> 22 : parchemin (frontières colorées selon les royaumes, écus, armées, boussole, sirènes) lisible, bascule 3D sans trou ni scintillement observé sur des captures fixes ; Paris/Londres/Rome/Venise/Moscou/Le Caire : maquettes riches, ombres cohérentes.
- Neige/hiver : sol et arbres bien saisonniers (winter_paris60.png), pluie et neige actives (364/495 particules).
- Alpes/Pyrénées : relief exagéré et enneigé spectaculaire hors rochers ; mer et Bosphore plausibles ; Moscou/steppe cohérents.
- Compteurs de dump sains (pas de NaN, pas d'erreur de chargement).

## Non testé / limites
- Popping et scintillement au zoom (captures fixes uniquement, pas de rejeu de descente) ; fondus de zoom non jugés.
- Oural réel non atteint (la carte s'arrête à 45 E : `east250`/`east_far` vus seulement en parchemin) ; Mont Athos, Islande non vus.
- Armées en mouvement, convois vivants, brouillard de guerre, frontières de diplomatie selon relations (une seule faction jouée : France).
- Automne (`autumn_paris100`) et `sea_far` captures faites mais non analysées en détail.
