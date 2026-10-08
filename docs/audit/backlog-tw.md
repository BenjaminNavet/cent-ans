# Backlog Total War — idées du joueur + compléments de l'orchestrateur (nuit du 24-25/09)

Liste vivante. Chaque idée devient un lot dans `docs/archive/chantiers.md` une fois planifiée. (J) = demandé par le joueur.

## Bataille — « champ de bataille vivant »

### Sang, blessures, morts
- (J) Sang : gerbes à l'impact (particules GPU), flaques et traînées au sol (décalques persistants), taches sur les figurines (masque dans le shader VAT), éclaboussures sur les chevaux. Réglage « Sang » : désactivé / modéré / complet.
- (J) Démembrements : têtes et membres détachés sur les coups critiques (hache, épée à deux mains, boulet, charge lourde), en variante de clip de mort VAT + petits maillages détachés (morceaux physiques de courte durée). Désactivé par défaut en réglage « modéré ».
- (J) Chutes : morts variés (en arrière, à genoux, projeté), chute de cheval du cavalier, cheval qui s'effondre en entraînant son cavalier ; cadavres persistants (MultiMesh figé) pendant toute la bataille, avec des flèches plantées.
- Blessés qui rampent et soldats qui fuient en jetant leur arme (déroute lisible).

### Physique et masse
- (J) Collisions de cavalerie : la charge a une masse. Les fantassins légers sont projetés et renversés (impulsion physique puis animation au sol), les chevaux ralentissent dans la masse, les piques et les pieux arrêtent et désarçonnent. Les règles (pertes, choc) restent dans core/ ; le rendu en tire des événements « impact » par soldat.
- Poussée des lignes (pushing), formations qui se déforment au contact, bords de formation qui s'enroulent.
- Duels appariés (matched combat) pour les généraux et les champions, en plans rapprochés.

### Masse et spectacle
- (J) Plus de modèles par unité : tailles d'unité (Petite / Normale / Grande / Ultra, ×0,5 à ×2,5), avec un LOD agressif (impostors au-delà de 300 m) pour tenir les FPS.
- (J) Plus de flèches : vraies volées (des milliers de projectiles en MultiMesh ou particules GPU, trajectoires balistiques), sifflement, flèches plantées au sol, dans les boucliers, les pavois et les corps ; flèches enflammées ; carreaux d'arbalète ; pluie de flèches qui obscurcit le ciel à Crécy.
- Nuages de poussière de la cavalerie, terre projetée par les sabots, bannières et fanions qui flottent au vent (vent de la météo).
- Artillerie : bombardes et trébuchets, avec projectiles, traînées de fumée, cratères, soldats projetés.
- Sièges : échelles, tours de siège, bélier, porte qui cède, huile bouillante, défenseurs sur les remparts.

### Caméra, son, lisibilité
- Caméra cinématique (Kill cam, ralenti, suivi d'un soldat), vue du général, discours avant la bataille (texte + voix synthétisée ou texte seul).
- Cris et jurons par unité (barks), chocs métalliques spatialisés, galop qui fait trembler, cor de retraite.
- Écran de fin de bataille détaillé (pertes par unité, tués par unité, héros), relecture (replay).

## Carte de campagne (à améliorer elle aussi)

- Armées représentées par des figurines animées (général à cheval + porte-étendard) qui marchent sur la carte, et flottes avec des navires.
- Villes et colonies en 3D qui grandissent avec le niveau (murailles, cathédrale, marché) ; fumées de cheminée ; champs et vignes cultivés autour, qui changent avec les saisons.
- Saisons visibles : neige l'hiver, blés dorés l'été, arbres roux l'automne ; météo sur la carte (pluie, nuages qui passent, ombres de nuages).
- Vue stratégique en carte parchemin enluminée au zoom maximal (TW Three Kingdoms / Medieval II).
- Vie ambiante : oiseaux, bateaux sur les fleuves, charrettes sur les routes commerciales (C5), moulins qui tournent.
- Frontières de faction lumineuses, teinte de territoire, contrôle et dévastation visibles (terres brûlées après une chevauchée, villages en ruine).
- Écran d'avant-bataille à la TW (rapport de forces, terrain, renforts, choix auto-résoudre / combattre / retraite), écran de siège (tours restantes, équipements).
- Suivi des mouvements de l'IA en fin de tour (caméra qui suit, option d'accélération).
- Portée de mouvement en surbrillance, chemin prévisualisé avec les tours (M4, mouvement libre).
- Événements illustrés en plein écran avec musique (enluminures existantes), cinématiques légères pour les grands moments (Crécy, peste noire, Jeanne d'Arc).

## Villes emblématiques (landmarks, à la TW : Rome, Constantinople)

- (J) **Paris réaliste**, sur la carte de campagne et en bataille de siège. État vers 1340-1380 :
  - la Seine avec l'île de la Cité et l'île Notre-Dame/île aux Vaches ;
  - Notre-Dame (façade à deux tours, flèche, arcs-boutants), la Sainte-Chapelle, le palais de la Cité ;
  - le Louvre de Philippe Auguste (donjon) et le Louvre de Charles V ;
  - les ponts habités (Grand-Pont, Petit-Pont) et le Châtelet ;
  - les enceintes de Philippe Auguste, puis de Charles V (rive droite, Bastille à partir de 1370) ;
  - les halles, les quais, Montmartre au nord.
  Sources : plans historiques du domaine public (plan de Bâle, restitutions), Wikimedia.
- (J) **Ensuite (lot L2, fait)** : Londres (la Tour, Old St Paul's, London Bridge, Westminster), Avignon (palais des Papes, pont Saint-Bénézet, remparts datés), Calais (port fortifié, Rysbank, Étape), Rouen (cathédrale, Saint-Ouen, château, Gros-Horloge), Bordeaux (Saint-André, Pey-Berland, Ombrière, Grosse Cloche, port de la Lune), Bruges (beffroi et halles, canaux, Notre-Dame). Gabarits paramétrés dans `landmark_monuments.py` ; suivi `docs/archive/chantiers.md`. Reste : toiles de fond de siège pour ces villes.
