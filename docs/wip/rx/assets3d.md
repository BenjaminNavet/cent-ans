# RX assets3d — critique directeur artistique 3D

## Verdict
- Forces : références d'assets saines (7 627 chemins de `data/` contrôlés, 706 assets du manifeste DN : 0 fichier absent, hors 1 chemin) ; soldats bleus/France lisibles et cohérents en siège ; pierre des murailles et fenêtres crédibles ; pack modèles `dn` hébergé (hors git) avec LODs (lod0/1/2).
- Faiblesses : un bâtiment générique « mange » la caméra en gros plan de bataille (toit étiré, rayures) ; échelles de siège en boîtes orange non texturées, nettes face aux murs réalistes ; poids des textures lourd (1,0 Go textures, 1,3 Go modèles, trois tableaux « hi » de 40 à 80 Mo) ; restes DN connus non traités.
- Carte de près (ville) : non vérifié en gros plan (capture à distance seulement, villes = blobs blancs de maquettes à cette échelle).

## Constats

### [majeur] bug — Chemin cassé : cartes de roseaux d'eau douce
**Constat** : la texture de carte des roseaux ne se charge pas ; les roseaux d'eau douce n'ont pas leur texture.
**Preuve** : `data/map/map_freshwater.json:23` pointe `res://assets/textures/vegetation/dn/card_reed_phragmites.png` ; le dossier `dn/` n'existe pas, le fichier est dans `vegetation/dn_cards/` (cf. `data/art/dn_cards_manifest.json:54`). Lu par `game/scripts/map/freshwater_layer.gd:148`. Seul chemin cassé sur 7 627.
**Correction proposée** : remplacer `dn/` par `dn_cards/` ligne 23.
**Coût** : S

### [majeur] bug — Caméra de gros plan de bataille à l'intérieur d'un bâtiment
**Constat** : avec `--closeup`, l'image est un pan de toit bleu-gris occupant tout l'écran ; les ardoises sont des rayures étirées avec des coutures horizontales visibles (texture non répétée proprement, UV étirées), la bataille est masquée.
**Preuve** : `/private/tmp/claude-501/rx-shots/assets3d/battle_close.png` (bataille d'Île-de-France, hameau). 
**Correction proposée** : exclure les bâtiments de la zone caméra du gros plan (ou masquer/transparence des bâtiments entre caméra et sujet) ; contrôler les UV du toit dans `game/shaders/building_pbr.gdshader` (échelle triplanaire en monde plutôt qu'UV du modèle).
**Coût** : M

### [majeur] finition — Échelles de siège procédurales, teinte orange unie
**Constat** : trois grandes échelles en boîtes lisses orange, sans bois ni usure, détonnent avec murs et soldats semi-réalistes ; également très épaisses.
**Preuve** : `siege.png` ; `game/scripts/battle/battle_siege.gd:56` `_make_ladder` (maillage procédural) ; `dn_catalog_battle.json` liste `siege:ladder` à générer.
**Correction proposée** : matériau bois (albedo+normal du pack textures bâtiments, teinte bois vieilli) et barreaux plus fins ; ou ingérer un glb `ladder` (catalogue DN).
**Coût** : S (matériau) / M (glb)

### [mineur] finition — Défenseur figé en pose statique sur le mur
**Constat** : un soldat debout, immobile, pieds flottants sur la face du mur (coin haut droit), pose de statue, éclairé comme un sprite.
**Preuve** : `siege.png`, haut droite.
**Correction proposée** : vérifier le placement (hauteur de chemin de ronde vs soldat) dans `battle_siege.gd` ; utiliser un modèle de défenseur cuit GA3.
**Coût** : S

### [majeur] conception — Poids des textures chargées
**Constat** : 1,0 Go de textures dont tableaux « hi » (`tx_campaign_bg_albedo_array.jpg` 80 Mo, parcelles 77 Mo, normales 46 et 40 Mo) et 10 tableaux de bataille de 16-17 Mo ; au moins 600 Mo sur disque joueur et pression VRAM au 2k.
**Preuve** : `find game/assets -size +15M` ; `game/scripts/map/texture_quality.gd` (bascule 1k/2k).
**Correction proposée** : vérifier que les hi ne sont chargés que sur machine capable (déjà prévu) et les sortir du paquet par défaut ; mesurer la VRAM au 2k.
**Coût** : M

### [mineur] finition — Restes DN connus
**Constat** : fragment de chaume sur le glacier, `town_port_harbour` encore TRELLIS 2, 7 villes refusées (ancien modèle gardé : castle_west, castle_med, town_iber, city_hansa…), archer_3_jack et fig_sled_driver_north non cuits, imposteurs d'arbres, roche `cliff_limestone` manquante.
**Preuve** : `docs/wip/dn-direction-artistique.md` § Clôture.
**Correction proposée** : traiter après les majeurs ; castle_west (château le plus visible, France/Angleterre) en premier.
**Coût** : M

### [mineur] finition — Placeholders de faune
**Constat** : `data/map/map_fauna.json` utilise `folk/horse.glb`, `folk/cow.glb`, `folk/sheep.glb` comme `placeholder` (le vrai modèle vient du pack `dn/fauna`, 62 animaux au manifeste).
**Preuve** : `data/map/map_fauna.json:70,81`.
**Correction proposée** : vérifier en jeu que le glb final est utilisé hors repli ; sinon câbler.
**Coût** : S

### [mineur] finition — Nuages blancs opaques sur la carte
**Constat** : blobs blancs durs (nuages/maquettes de ville à distance) donnent un aspect « coton » et masquent des provinces (Bretagne, Angleterre).
**Preuve** : `town.png`.
**Correction proposée** : adoucir bords et opacité des nuages à cette distance (`game/shaders/map_atmo_cumulus.gdshader`) ; à valider par le rôle carte.
**Coût** : S

## À ne pas changer
- Soldats France bleu/fleurs de lys en siège (échelle, armure, animation d'escalade) ; murs en pierre du siège ; chaîne manifeste DN + hébergement `models-v2` ; vérification des chemins : tout est cohérent hors constat 1.
- Limite : captures non faites en gros plan ville (campagne) ni cavalerie : à compléter.
