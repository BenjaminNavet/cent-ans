# Testeur visuel du client Bevy (17/10, tête 9395b327a)

Méthode : client release `--offscreen`, muet, port BRP 15742 ; menu par script (`click`), puis BRP (touches, molette, `cent_ans/dump`, captures via la caméra `UiCamera`, qui compose tout). Captures : `out/tester/*.png` (34, 1280x800). Le clic souris BRP ne marche pas dans le menu ni sur les boutons de panneau ; il marche sur la carte de campagne.

## 0. Écran de choix de faction (signalé par le joueur)
Reproduit. `out/tester/fs_02_tab_all.png`, `fs_03_england.png`, `fs_05_lithuania.png` (onglet « Toutes les factions »). La carte (`menu.faction_map` : loaded, 900x782, 213 provinces) existe mais est hors écran : 65 pastilles de royaume (11 lignes) + 4 de rang occupent toute la hauteur, la carte est repoussée sous la ligne de flottaison (le bas ne montre que la liste « Sans terre / Saint-Empire… ») ; pas de défilement. Choisir Angleterre / Bourgogne / Lituanie ne change que la fiche de droite : aucune surbrillance de terre visible, aucun cadrage. Onglet « Départs recommandés » (`fs_01_france_default.png`) : pas de carte du tout, 3 cartes affichées alors que le dump `faction_choice.cards` en liste 4 (Croisés manque). Navarre : `fm:fac_navarre` inexistant (pas de terre en 1337, normal). Hypothèse : mise en page verticale sans contrainte de hauteur pour le bloc de filtres (`menu/faction_select.rs`, `faction_map.rs` MapBody) ; à vérifier aussi à 1920x1080.

## Défauts
### Bloquant
- Sol blanc-gris en campagne (`c03b_d200_retry.png`, `c04_d60.png`, `p03_province.png`, `p04_army.png`) : Île-de-France, après un cycle de zoom (2600 -> 190 -> 600) ; seuls les patchs de forêt flottent. Le même lieu lancé directement à 200 est correct (`f01_paris_200_fresh.png`), Vilnius à 190 et 25 aussi (`l02`, `l03`). Déclencheur probable : j'avais panné la caméra à z=0 (bord de la carte, `focus.y=0.0`) à 2600 avant de zoomer : hauteur / pages de sol hors carte, ou colormap perdue (dump `campaign_terrain` dit pourtant ready, missing_pages 0). Note : tout joueur qui scrolle au bord de la carte l'obtient peut-être.
- Carte de la sélection de faction hors écran (section 0).
### Laid
- Guyenne (`g01_guyenne_400.png`) : sol marron foncé uniforme, rupture brutale à la frontière avec le vert de France ; pas d'estuaire de la Gironde, côte rectiligne ; irréaliste pour les Landes/Bordelais au printemps.
- Normandie (`n01_normandie_250.png`) : beau rendu, mais pas de Cotentin, Caen sur le rivage, pas d'estuaire de la Seine.
- Bataille de Crécy (`b01_crecy_start.png`, `b02_crecy_near.png`, `b03_crecy_front.png`) : caméra d'ouverture sur un pré vide à ~200 m des troupes ; régiments en simples files fines d'une rangée, figurines de 1-2 px à 84 m ; terrain plat, sans arbre ni relief ; plaques beige rectangulaires. HUD annonce « Pluie » alors que `battle_weather.weather` = clear et que la pluie est active (incohérence).
- Siège d'Avignon (`s01_siege_avignon.png`) : plan d'eau rectangulaire à bords droits (haut droite), cubes gris pâle épars devant les troupes, deux traits rouges flottants au bord droit, ville réduite à une ligne de tours coniques.
- Maquettes de villes décalées des libellés (`h01_flandre_300.png` : Bruges, Lille, Gand) ; Vilnius (`l03_vilnius_25.png`) est une grande église gothique en brique sur un socle en cuvette, route qui s'arrête dans le vide, arbres en gros blocs gris : anachronique pour la Lituanie païenne de 1337.
- Parchemin (`c02_far.png`) : très chargé (halos fluo de frontières, points noirs d'arbres), étiquettes qui se chevauchent (Paris/620, Cologne/Liège).
- Diplomatie F3 (`p01_diplomacy_f3.png`) : bloc « Opinion » coupé, boutons rognés derrière les onglets ; noms de puissances tronqués.
- Technologies F4 (`p02_tech_f4.png`) : cases à cocher vides (glyphe absent), colonnes coupées à droite, libellés tronqués (« Archers à p… »).
### Mineur
- Deux infobulles empilées illisibles (armée + « Le hêtre », `p05_recruit.png`, `p06_council.png`) ; la seconde reste collée.
- Panneau d'armée recouvre la barre « l'ost de… » (`p04_army.png`).
- Échap en bataille n'ouvre pas la pause (`z01_pause_battle.png`, le temps continue) alors que la pause de campagne marche (`z02_pause_campaign.png`).
- Touche J (conseil) sans effet via BRP `KeyJ` (non confirmé, peut venir de l'injection). Recrutement (bouton Recruter) non atteignable par clic BRP.
- Codex (`k01_codex.png`) : fond noir pur, volet de droite vide.
- Journal : 0 ERROR ; WARN : `bevy_light` ShadowLodOrigin (x5, ombres ponctuelles sans caméra fenêtre sous --offscreen), `bevy_pbr::prepass` normales manquantes (x2, un maillage sans normales), bindless metal. Son : muet, rien à signaler.
- Tours : 4 fins de saison via Maj+Entrée (T0 -> 4, 1338) sans erreur ; Entrée seule ouvre la confirmation puis ne confirme pas au second Entrée.

## Notes /10 (réalisme / graphismes / UI)
Campagne France 600 : 7/7/7 ; Paris 200 frais : 8/8/7 ; Paris après zoom : 2/3/6 ; Normandie : 7/8/7 ; Guyenne : 4/6/7 ; Flandre : 8/8/7 ; Vilnius 600/190/25 : 6/8/7 ; parchemin : 6/6/5 ; Crécy : 4/5/7 ; siège : 4/5/7 ; sélection de faction : -/6/3 ; diplomatie, tech : -/-/5.

## Meilleures captures (à montrer)
f01_paris_200_fresh, h01_flandre_300, n01_normandie_250, l01_vilnius_600, l03_vilnius_25, c01_start, p03_province, menu.
Défauts à montrer : fs_02_tab_all, c03b_d200_retry, g01_guyenne_400, s01_siege_avignon.
