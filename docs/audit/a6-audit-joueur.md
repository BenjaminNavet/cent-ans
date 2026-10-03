# A6 — Audit joueur (03/10)

Méthode : deux parties pilotées à la souris (`zz_audit_pilot.gd`, non commité), France, 1920×1080, 12 saisons chacune (printemps 1337 → printemps 1340) : survols, infobulles, panneaux, recrutement, construction, diplomatie, recherche, bataille de campagne, siège de Bordeaux, menus. Référence : Total War (TW) et Crusader Kings (CK). Mesures d'équilibre longues : sonde A2 relancée (4 graines × 200 tours), résultats en bas de page.

Gravité : **B** bloquant ou faux, **M** majeur (gêne nette), **m** mineur. Lot = lot de correction (voir `docs/wip/a6-audit-joueur.md`).

## Mécanique et équilibre

| # | Constat | Gravité | Proposition | Lot |
|---|---|---|---|---|
| M1 | Prévision de bataille fausse : 620 contre 540, « Défaite presque certaine, 3 % », bataille livrée gagnée (pertes 25 % contre 35 %). Déjà signalé en Q8. | B | La prévision et la résolution automatique doivent partager la même formule ; calibrer la sigmoïde pour que des forces voisines donnent 35-65 %. | L1 |
| M2 | La barre de rapport de forces affiche environ 45/55 alors que le texte dit 3 % : la pente est trop raide. | M | Même source que M1 ; texte et barre dérivés de la même probabilité. | L1 |
| M3 | Siège de Bordeaux : murailles niveau 5, brèche 0 %, et l'assaut affiche « chances ≈ 100 % » contre 340 hommes. Échelles et bélier « prêts » dès le premier tour. La bataille de siège est gagnée avec les murs à 100 % et la porte tenue. | B | L'assaut sans brèche doit être coûteux et incertain. Engins construits en 1 à 3 saisons selon le niveau des murailles. La prévision d'assaut tient compte des murs. | L2 |
| M4 | Économie noyée : 60 000 ₶ au départ, +4 346 par saison, bâtiment à 300 ₶, unité à 460-1 150 ₶. 106 678 ₶ après 12 tours sans dépenser. Une rançon de 18 600 ₶. L'argent ne compte pas. | B | Remettre le trésor à l'échelle : trésor de départ ≈ 2 à 3 saisons de revenu, bâtiments à 4-12 saisons de rendement, rançons plafonnées à une fraction du revenu annuel. | L3 |
| M5 | Édouard III capturé à la première bataille du printemps 1337, puis libéré contre rançon automatiquement. | M | Probabilité de capture du chef selon la déroute et la cavalerie adverse, plafonnée ; pas de capture du souverain en premier tour hors déroute totale. | L3 |
| M6 | Diplomatie au hasard : une proposition affichée à 65 % est refusée. | M | Règle déterministe à la CK : acceptée si le score ≥ 0, et le chiffre affiché est le score avec ses facteurs. | L5 |
| M7 | Recherche : « Aucune recherche en cours — 42 points par tour perdus ». | M | Points accumulés quand rien n'est choisi (ou choix automatique du moins cher), plus une file de 3 recherches. | L4 |
| M8 | Une seule construction à la fois par colonie. | m | Garder la règle mais l'afficher ; file de construction de 2 à 3 entrées qui démarrent l'une après l'autre (TW). | L7 |
| M9 | La France n'a qu'une armée propre au départ ; les vassaux tiennent le reste. Les plaques « 240 » sont partout. | m | Expliquer dans l'infobulle de l'armée vassale (« armée de X, vassal ») ; regroupement visuel. | L8 |

## Interface et ergonomie

| # | Constat | Gravité | Proposition | Lot |
|---|---|---|---|---|
| U1 | Menu principal à 12 entrées. | m | Regrouper les batailles (personnalisée, historiques, démos, rejouer) dans un sous-menu « Batailles ». | L11 |
| U2 | Choix de faction : larges marges de parchemin vides, texte minuscule. | m | Mise en page qui occupe la largeur ; corps de texte ≥ 16 px à 720p. | L11 |
| U3 | Citation d'écran de chargement hors contexte (Serge de Radonège à Dimitri Donskoï, 1380, pour la France). | m | Citations filtrées par culture ou faction du joueur, repli générique. | L11 |
| U4 | Barre du haut : 9 icônes rondes avec seulement la lettre du raccourci. | M | Infobulle avec nom, raccourci et état ; libellé court sous l'icône à 1080p. | L8 |
| U5 | Recherche tronquée en « Aucune recher... » dans la barre du haut. | m | Libellé court (« Recherche : — ») et détail dans l'infobulle. | L4 |
| U6 | Journal : 207 entrées après un tour, puis 40 à 43 par tour. | M | Filtrer sur ce qui concerne le joueur (ses provinces, ses voisins, ses alliés, ses ennemis) ; le reste va dans un onglet « Monde » replié. | L6 |
| U7 | Lettres sans rapport poussées à la France (croisade de Gaza, alliances Écosse–Leinster). | M | Même filtre de pertinence que U6 pour les lettres et alertes. | L6 |
| U8 | La pile de lettres recouvre le bouton de fermeture de la barre d'agents, qui reste ensuite par-dessus le panneau de province. | M | Pile ancrée sous la barre du haut, à droite, avec une zone réservée ; la barre d'agents se ferme avec Échap et en ouvrant un panneau. | L6 |
| U9 | Cliquer l'écu ou le nom de la faction dans la barre du haut ne fait rien. | m | Ouvre la fiche de faction (vue d'ensemble du royaume, TW et CK). | L8 |
| U10 | Fenêtres empilées : le rapport de saison passe sur la fenêtre de capture ; la citation du conseiller passe sur le rapport ; le rapport réapparaît après une bataille, en milieu de tour. | M | File de fenêtres modales : une à la fois, les décisions avant les rapports ; le rapport de saison seulement en début de tour. | L6 |
| U11 | Panneau de colonie (Paris) étroit, avec un défilement dans un défilement. | M | Un seul défilement ; panneau plus large. | L7 |
| U12 | Liste de construction : raisons d'indisponibilité serrées dans une colonne rouge étroite (Q8 n'avait corrigé que le recrutement). | M | Même traitement que le recrutement : raison sur sa propre ligne, pleine largeur. | L7 |
| U13 | Liste de recrutement : unités d'autres cultures en tête et grisées (Mamelouks « réservé à d'autres factions », Chevaliers bretons). | M | Disponibles d'abord ; celles « réservé à d'autres factions » masquées ; celles qui attendent une technique ou un bâtiment groupées sous « Bientôt ». | L7 |
| U14 | Panneau de colonie (sobre) et panneau de province (enluminé) dans deux styles, séparés. | m | Harmoniser le style ; à terme, un panneau de province TW avec une grille d'emplacements. Pour cette passe, style seulement. | L7 |
| U15 | Fenêtre de diplomatie qui déborde à droite (« + Clau », « + Ajoute ») ; cartes de clause coupées un mot par ligne (« Accord commerci al »). | M | Largeur minimale des colonnes et retour à la ligne par mot, sans coupure. | L5 |
| U16 | Commerce sans bouton visible (touche V seulement). | m | Bouton dans la barre du haut ou dans la diplomatie. | L5 |
| U17 | Fenêtre des agents haute et presque vide. | m | Hauteur ajustée au contenu. | L6 |
| U18 | Le survol d'une province affiche une large barre en bas au centre. | m | Barre plus discrète, à côté du curseur ou dans un coin. | L9 |
| U19 | Une armée en garnison à Paris ne se sélectionne pas d'un clic : le clic ouvre Paris (12 échecs sur 12). | B | Priorité à la plaque d'armée sous le curseur ; un clic sur la ville dont une armée est présente sélectionne l'armée, double clic ouvre la ville (TW). | L8 |
| U20 | Cloche de fin de tour sans effet ni message quand le menu pause est ouvert. | m | Cloche désactivée avec infobulle pendant une fenêtre modale. | L6 |
| U21 | Ellipse jaune de sélection de Paris démesurée, qui recouvre la ville. | m | Taille liée à l'emprise de la ville, plafonnée. | L8 |
| U22 | Bataille : journal, bandeau de déploiement, ordres du chef, formations de groupe, cartes d'unités et minicarte se disputent l'écran ; la harangue cache le panneau des formations. | M | Harangue en haut au centre ; journal replié par défaut ; ordres du chef et formations regroupés. | L12 |
| U23 | Compteurs d'alertes de bataille à ×101 et ×103 (« Unité en déroute », « Flanc ou dos attaqué »). | M | Une alerte par unité et par événement, oubliée après 10 s ; plafond d'affichage « ×9+ ». | L12 |

## Carte de campagne

| # | Constat | Gravité | Proposition | Lot |
|---|---|---|---|---|
| C1 | On ne distingue pas le territoire du joueur : le remplissage par position (RJ-d, ADR 0175) est trop faible à vue moyenne. | M | Alpha du remplissage relevé à vue moyenne et lointaine (TW : teinte nette, CK : carte politique). L'ADR 0155 garde le rouge réservé aux ennemis. | L9 |
| C2 | Des dizaines d'étiquettes de rivières en italique bleu ; noms de provinces pâles. | M | Rivières nommées seulement en vue proche et les grandes seulement en vue moyenne ; noms de provinces plus contrastés. | L9 |
| C3 | Trop d'écus : un par ville, un par plaque d'armée ; plaques d'armée grandes. | m | Plaques d'armée réduites de 20 % ; écus de villes mineures masqués en vue moyenne. | L9 |
| C4 | Palette d'été qui rend le nord de la France brun-orangé, presque désertique. | M | Été plus vert pour les biomes tempérés ; le brun réservé au sud sec. | L9 |
| C5 | Hiver très gris. | m | Hiver plus clair et plus froid (neige légère), moins de gris. | L9 |
| C6 | Traînées de pluie sur tout l'écran. | m | Densité réduite et pluie limitée aux zones de météo. | L9 |
| C7 | Vue parchemin encombrée d'icônes de nuages météo. | m | Icônes météo masquées en vue parchemin, ou une par région. | L9 |
| C8 | Carte des religions : le bleu de l'obédience d'Avignon trop proche du bleu de l'islam. | m | Islam en vert (convention), Avignon en bleu. | L9 |
| C9 | Texture du sol en camouflage tacheté à vue moyenne (ombres de nuages + mélange de sols). | m | Ombres de nuages atténuées en vue moyenne. | L9 |

## Bataille

| # | Constat | Gravité | Proposition | Lot |
|---|---|---|---|---|
| B1 | Batailles très courtes (2 à 5 min, contre 10 à 20 min dans TW) ; petites unités (60 à 120 hommes). | M | Moral et cohésion plus résistants, dégâts réduits de façon à doubler la durée ; mesure par la sonde de durée. | L13 |
| B2 | Sol plat, en carreaux, au même aspect tacheté par les ombres de nuages. | M | Ombres de nuages plus douces et plus larges ; variation de sol à plus grande échelle. | L12 |
| B3 | La caméra de déploiement part basse et loin des troupes. | M | Départ cadré sur l'armée du joueur, à hauteur moyenne, face à l'ennemi. | L12 |
| B4 | À fort zoom, les unités sont des points sous de grandes icônes. | m | Icônes plus petites avec la distance ; masquées en dessous d'une taille d'écran. | L12 |

## Performance

| # | Constat | Gravité | Proposition | Lot |
|---|---|---|---|---|
| P1 | 60 i/s partout (synchro verticale). | — | Rien. | — |
| P2 | 6 821 appels de dessin au zoom maximal, contre environ 560 en vue normale ; jusqu'à 5 M primitives au plus près. | M | Identifier les couches qui explosent (villes 1:1, arbres, figurines) et regrouper ou réduire leurs distances de détail. | L10 |
| P3 | Qualité « ultra » à 15 i/s (Q8). | m | Même lot : profil ultra plafonné. | L10 |
| P4 | Pas d'erreur de script ; fuites de ressources à la fermeture. | m | Libérer les ressources orphelines signalées à la sortie. | L10 |

## Remise en cause d'ADR

- **ADR 0155 (rouge réservé aux ennemis)** : maintenu, mais le remplissage du territoire propre doit se voir (C1). Pas de nouvel ADR, simple réglage.
- **Prévision de bataille** : la prévision doit lire la même formule que la résolution automatique (M1). ADR à écrire par L1 si la formule change.
- **Économie** : la remise à l'échelle (M4) change les ordres de grandeur ; ADR à écrire par L3.

## Mesures d'équilibre longues (sonde A2)

`balance_probe` sur `main` (cb3dd3187), 4 graines × 200 tours, plus `balance_probe rt 4` (3D). Une saison = un tour.

| Indicateur | A2 | Maintenant | Cible | Lot |
|---|---|---|---|---|
| Milice dans les recrutements IA | 99,8 % | 64,9 % (20-22 types recrutés) | < 40 % | L3 |
| Auto-résolution contre 3D (accord sur le vainqueur) | — | 17/20 ; reste cavalerie contre archers (Crécy : auto 100 %, 3D 0/4) | ≥ 19/20 | L1 |
| Changements de propriétaire | 12/partie | 254/partie, 93 factions détruites | — | — |
| Révoltes | 1/partie | 14,2/partie, 67 % en province occupée ; `peace_of_god` choisi 91 % | 4-10 | L3 |
| Recherche (France, tour 200) | 45/45 | 40-43/45 | plus lente | L3 |
| Revenu France / Angleterre | 2-3,7× | 2,8-3,0× (une graine 0,9) | < 2× | L3 |
| Bourgogne | 3 prov. | trésor ≈ 0, solde structurel −535/saison | solde > 0 | L3 |
| Banqueroutes | — | 1,62 par faction et par décennie | < 0,5 | L3 |
| Bâtiments jamais construits | 10/30 | 9/30 (armurerie, forge, écuries, buttes de tir…) | ≤ 3 | L3 |

Économie au tour 0 (revenu brut / solde net par saison) : France 27 686 / +4 254, Angleterre 19 185 / +860, Bourgogne 3 206 / −535. Le trésor de départ de la France (60 000 ₶) vaut 14 saisons de solde net ; un bâtiment moyen (1 366 ₶) coûte 5 % du revenu brut. Rançons : médiane 3 300 ₶, max 51 260 (facteur richesse × prestige). Constat M4 confirmé côté joueur ; l'IA, elle, dépense tout.
