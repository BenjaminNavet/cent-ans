# Audit de fond — expérience joueur et présentation (29/10/2026)

Auteur : audit Opus (lecture seule), boucle juge 2, itération 2. Périmètre : prise en main (30 premières minutes),
architecture de l'interface, retour et causalité, interface parchemin et son unité avec la bataille, audio,
accessibilité, narration. Le rendu de la carte de campagne relève de l'audit `2026-10-29-visuel-campagne.md`.
Le lot TUTO en cours (worktree, base 99d672914 : 4 étapes de didacticiel, réserve de recrutement, barre du haut serrée,
champ prestige supprimé) est pris en compte : ses constats ne sont pas recomptés comme ouverts.

Preuves : dumps et captures dans `out/audit/xp/` (hors git), dumps de fumée du 29/10 18:4x (`out/smoke/*.json`, main
99d672914), lecture du code et des données. Les captures ne servent qu'au jugement visuel.

---

## 0. Ce que j'ai joué

J'ai joué trois parties par script : `client/run_bg.sh`, hors écran, muet, graine 1337, 1280×800. Les scripts sont `out/audit/xp/s1.txt`, `s2.txt` et `s3.txt`.

1. **Menu, choix de faction, Angleterre, Commencer** (`d1`–`d4`, captures `c1_menu`, `c2_fs`).
   - « Commencer » ouvre d'abord la modale « Jouer le didacticiel ? ».
   - Le briefing et le guide ne viennent qu'après.
2. **France, cinq saisons sans autre geste que finir la saison** (`f0`–`f5`, `f_rec`, `f_confirm`, `f_chron`, captures `c4_rec`, `c5_t3`, `c6_chron`).
   - C'est le comportement d'un débutant qui ne sait pas quoi faire.
   - La partie tient, mais le jeu ne l'a jamais relancé utilement.
3. **Bataille mise en scène depuis la campagne, Angleterre** (`b0`–`b2`, capture `c8_result`).
   - Le dialogue d'avant bataille est riche : verdict « Victoire presque certaine » (96 %), généraux avec portraits, site, météo.
   - Avec le crochet de test `CENT_ANS_FLOW_SIM_TO_END`, la bataille se termine sans vainqueur. Le joueur lit alors seulement « Vous avez quitté le champ de bataille. », pendant que le journal note « Notre ost refuse la bataille près de Londres et se replie (moral −10) », alors qu'il avait cliqué « Combattre ».
   - Ce chemin vient en partie du crochet de test. Mais il montre que **l'abandon d'une bataille n'est expliqué nulle part** : aucune conséquence affichée, et le texte se contredit.
   - Le HUD de bataille en 3D est jugé sur la fumée du jour (`out/smoke/battle.json`) et sur le code, pas sur une capture. La file des slots lourds était saturée : environ une heure d'attente par lancement.
4. **Dumps de fumée du jour** (`out/smoke/turns_1.json`, Angleterre, tour 3) en complément.

**Relevé par saison (France, passif)** :

| Saison | Trésor | Solde | Mécontent. | Évènements du tour | Alertes | Widgets à l'écran | Guide |
|---|---|---|---|---|---|---|---|
| Printemps 1337 (t0) | 16 000 | +10 542 | 11 % | 0 | 3 | 64 | étape 1/14 |
| t1 | 27 843 | +9 899 | 14 % | **362** | 5 | 300 | étape 1/14 |
| t2 | 39 023 | +7 876 | 17 % | 51 | 4 | 298 | étape 1/14 |
| t3 | 48 167 | +6 244 | 20 % | 55 | 6 | 290 | étape 1/14 |
| t5 | 60 255 | **+201** | 24 % | 67 | 5 | 302 | étape 1/14 |

**Moments de confusion relevés**
1. **Menu** (`c1_menu`) :
   - titres blancs sur un parchemin clair, peu lisibles (déjà signalé le 22/10, toujours là) ;
   - un **cadre vide** en haut à droite ;
   - « Edimbourg » cuit dans l'image, sans accent.
2. **Choix de faction** (`c2_fs`) :
   - la carte politique est un aplat sans aucun nom : on ne sait pas qui est qui ;
   - la fiche annonce « Revenu 26 639 ₶ / saison » alors que la barre du haut de la partie affiche **+10 542** : brut contre net, sans le dire ;
   - 139 factions.
3. **Guide** : la carte « Bienvenue » reste à l'étape 1/14 pendant cinq saisons. Rien ne relance le joueur qui l'ignore, et Entrée termine la saison par-dessus.
4. **Fin de saison** : la modale « Terminer la saison ? » apparaît **à chaque tour**, parce que l'alerte « 32 localités sans chantier » ne s'éteint jamais. Le garde-fou devient un clic réflexe.
5. **Tour 3** (`c5_t3`) :
   - la fenêtre Diplomatie **s'ouvre d'elle-même** (offre « pesante ») et couvre tout l'écran ;
   - la carte du guide la **recouvre** et masque la liste « Les puissances » et le champ de recherche ;
   - sous « En guerre contre », une rangée de boutons vides est coupée.
6. **Tour 5** (`c6_chron`) :
   - l'offre de missions s'empile sur la Chronique ouverte ;
   - deux missions quasi identiques sont proposées ensemble : « Prendre Ponthieu » (générique) et « Reprendre le Ponthieu » (de faction) ;
   - les récompenses disent « −10 **agitation** » quand la barre dit « **Mécontentement** » ;
   - l'alerte dit « 32 **colonies** » quand la pastille dit « Chantiers libres » et la cloche « 32 **localités** » ;
   - panneau ouvert, les pastilles d'alerte perdent leur libellé et ne montrent plus que des nombres (1, 1, 32, 1, 1).
7. **Le solde s'effondre de +10 542 à +201 en cinq saisons** sans aucune action du joueur. La seule trace est une ligne tronquée du fil : « Revenus : 25 008 ₶, entretien : 21 784 ₶… ». Le journal des causes du joueur est **vide** (`panel_chronicle.causes.journal_len = 0`) après 5 saisons, 87 lignes de journal et 33 lettres.
8. **Fil « Guerre » de la France** : « Angleterre déclare la guerre à **Germiyan** », « Foix-Béarn déclare la guerre à Germiyan ». C'est un beylik anatolien, présenté au même rang que sa propre guerre.
9. **Recrutement** (`c4_rec`) : le texte des cartes est coupé (« 6 disponibles (réserve… »). « Hommes 620 = » est en fait l'effectif de l'ost, pas une réserve (corrigé par TUTO).

**Clics des tâches courantes** (mesurés par script, ou comptés sur les noms de widgets du code) :

| Tâche | Actions | Remarque |
|---|---|---|
| Finir la saison | **2** (Entrée, Confirmer) + 1 à 2 fermetures de fenêtres ouvertes d'office | la confirmation est permanente (C4) |
| Bâtir dans une localité | 4 (province, onglet Bâtiments, localité, bâtiment) | × 32 localités au départ : environ 130 clics |
| Recruter une unité | 4 à 5 (ville, Recrutement, carte, Sceller la levée, fermer) | correct ; panier à la Total War |
| Comprendre pourquoi le solde baisse | 1 survol (composition, sans écart) puis Économie (M) ; aucune vue « depuis la saison passée » | la cause n'est jamais nommée (M2) |
| Changer l'impôt | 3 (écu de faction, onglet, taux) ou par province | deux chemins |
| Lire ce qui s'est passé | Journal (déplier), Fil (déplier), Chronique (F5, 4 onglets), lettres | quatre lieux (M1) |

---

## (a) Diagnostic classé par gravité

Échelle : **critique** (fait fuir le joueur dans les 30 premières minutes ou casse le sens du jeu), **majeur**
(ralentit, embrouille, affaiblit l'identité), **mineur** (finition).

### Critiques

**C1. Personne ne décide de ce qui mérite l'attention du joueur : neuf canaux parlent en même temps.**
- Constat. Au tour 3 en Angleterre (`out/smoke/turns_1.json`), voici ce qui sollicite le joueur en même temps :
  - **la cloche** : 9 alertes, dont 3 décisions de chronique bloquantes (*Le chroniqueur*, *Le raid de Cadzand*, *La défection d'un capitaine*), « 37 localités sans chantier », « score de l'acte 6 sur 100 », une offre diplomatique, une armée oisive et une recherche oisive ;
  - **le bandeau Conseiller** (« Une décision vous attend ») et **la carte « Que faire maintenant »**, qui portent le même texte ;
  - **le fil d'évènements** : 21 entrées, dont 6 de diplomatie étrangère (Gueldre, Leinster–Bourgogne) ;
  - **le journal** : 56 lignes, plus 197 lignes « Monde » ;
  - **16 lettres scellées en trois tours** ;
  - **3 toasts** et **le suivi de missions**.

  Le dernier tour a produit **57 évènements** (`hud.last_turn_events`). Une offre de mission a expiré sans que le joueur l'ait vue : le fil le dit (« L'offre de mission est restée sans réponse »).

  En France (partie 2), le **premier** tour produit **362 évènements**, puis entre 51 et 67 par tour. On passe de 64 à environ 300 widgets à l'écran dès le tour 1. Des fenêtres s'ouvrent d'office et se chevauchent : Diplomatie et guide au tour 3, missions sur la Chronique au tour 5.
- Cause racine. Chaque lot a ajouté son propre canal : l'avis du kit, le bandeau ADR 0752, le conseil UX2, la cloche, le fil, le journal, les lettres, le rapport de saison ADR 0281, l'ADR 0789 et les missions ADR 0304. Aucun budget d'attention ne les arbitre. La priorité est réglée canal par canal (`advisor_band` a des priorités, `ModalQueue` aussi), jamais entre les canaux. Le monde simulé, avec 139 factions jouables et 65 royaumes, produit un flux qui dépasse tout ce qu'un joueur peut lire. Et le filtre « intérêt » trie ce flux sans le résumer.
- Effet. Le joueur ne sait pas ce qui compte. Il apprend à tout ignorer, y compris ce qui compte vraiment (mission expirée, décision bloquante).

**C2. Le didacticiel apprend des boutons, pas le jeu ; il est contournable sans le savoir.**
- Constat.
  - Le guide (`data/tutorial/steps.json`, 14 étapes, 18 après TUTO) enchaîne des gestes : « cliquez sur la bannière », « ouvrez l'onglet Bâtiments », « choisissez une technique », « ouvrez la diplomatie (l'ouvrir suffit) ».
  - Il n'enseigne aucun arbitrage : pourquoi bâtir ceci plutôt que cela, quand faire la guerre, ce qui fait gagner l'acte.
  - Au tour 3 de la fumée (Entrée ×6), il est encore à l'étape `intro`. En France (partie 2), il reste à « étape 1 sur 14 » pendant **cinq saisons**. Rien n'empêche ni ne signale qu'on a sauté la saison sans lui.
  - Le didacticiel de bataille (`battle_prologue.json`) n'est proposé que depuis le menu Batailles, jamais à la première bataille de campagne.
  - Le briefing de départ (`transitions.json`) donne trois conseils génériques, identiques pour toutes les factions.
- Cause racine. Le guide a été conçu comme une visite de l'interface (port de F8, Godot), avec des conditions « panneau ouvert ». Il n'a jamais été pensé comme un scénario pédagogique avec un but de partie. TUTO ajoute des étapes (recrutement, siège, bataille, ravitaillement), mais garde ce modèle.

**C3. 139 factions jouables pour un contenu écrit pour 3 : l'identité se dissout dès le choix de faction.**
- Constat.
  - Le document de conception fixait 3 factions (France, Angleterre, Bourgogne). L'écran en propose 139 (`menu.factions_pitched`), sur trois onglets.
  - Le contenu propre (conseils du guide, missions de faction : 8 gabarits, voix du conseiller, listes de musique régionales) ne couvre que les trois maisons principales.
  - Un joueur qui choisit la Bohême ou Novgorod reçoit des conseils génériques, aucune mission propre et une musique « campagne » par défaut. Rien ne le prévient.
- Cause racine. On a ouvert le jouable sans définir de **paliers de contenu**. L'écran ne dit pas ce qui est écrit et ce qui est générique.

**C4. L'échelle de gestion produit des tâches vides : « 32 localités sans chantier » au tour 0.**
- Constat. Au tour 0, France a 10 provinces et **32 localités sans chantier** (`out/smoke/campaign.json`) ; au tour 3, l'Angleterre en a 37.
  - Bâtir passe par la localité : clic sur la province, onglet Bâtiments, choix de la localité, bâtiment, soit 4 clics par chantier (`prov:tab_buildings`, `prov:set:<settlement>`, `prov:build:<id>`).
  - Remplir les emplacements du départ demande donc environ 130 clics.
  - L'alerte permanente « 37 localités sans chantier » devient du bruit. Le conseil de saison (ADR 0789) relaie la même injonction.
- Cause racine. La granularité de la localité (ADR 0011, colonies) est bonne pour la simulation, mais l'interface ne propose ni délégation (intendant, file par province) ni vue agrégée « chantiers du royaume ».

### Majeurs

**M1. Fenêtres redondantes : environ 22 fenêtres, 10 lanceurs, la même information à 3 endroits.**
- Budget : trois endroits (onglet budget de Ma faction, aperçu d'Économie, Finances royales Maj+J).
- Commerce : deux endroits, plus la couche V.
- Cour : trois endroits (onglet Cour de Ma faction, personnage C, Conseil J en 2D et en 3D).
- Suzerain et vassaux : deux endroits (fiche de faction, Féodalité).
- Histoire : trois endroits (Journal, Fil, Chronique).
- Objectifs : trois endroits (O, suivi de missions, briefing).
- Raccourcis :
  - 4 lanceurs sans touche : Recrutement, Ma faction, Féodalité, Codex. K est codée en dur dans `codex/mod.rs:330` et échappe à la réassignation.
  - 3 raccourcis annoncés par F1 et câblés nulle part : B (avoirs), G (agents), L (encyclopédie).
- Cause : un port fidèle des panneaux Godot (un fichier par jalon, ADR 0002), puis des ajouts lot par lot sans refonte de l'arborescence.

**M2. La causalité existe dans le cœur, mais le joueur ne la voit pas.**
- Ce qui existe :
  - le journal des causes (`sim-campaign/src/causes.rs` : guerre déclarée, révolte, banqueroute…) ;
  - les décompositions de revenu (`income_breakdown.rs`) et de troubles (`population.rs:432`) ;
  - les raisons d'attitude de l'IA (`game-api/diplomacy.rs`) ;
  - les bulles de la barre du haut (`treasury_tooltip`, `budget_tooltip`).
- Ce qui manque :
  - Les causes vivent dans le 4ᵉ onglet de la Chronique. Au tour 3 anglais, `panel_chronicle.causes.journal_len = 0`, alors que la guerre est déclarée et que 57 évènements ont eu lieu.
  - Le rapport de saison, réglé sur « auto », ne s'ouvre que sur une perte, une prise ou une bataille. Une saison calme se résume en un toast (« rien à signaler »).
  - Preuve, en France passive : le solde tombe de **+10 542 à +201** en cinq saisons, parce que l'entretien monte jusqu'à 21,8 k ₶ pour 25 k de revenus (lignes du fil). Aucun rapport ne s'ouvre et aucune cause n'est enregistrée. Seule une ligne tronquée du fil en porte la trace.
  - Les lignes du fil sont tronquées au milieu de l'information (« Revenus : 17 846 ₶, entretien : 13 091 ₶… », « Traité entre… »).
  - Le score d'acte (« 6 sur 100 ») ne dit pas ce qui le fait monter.
- Cause : on a bâti la causalité comme une **base** (consultable), pas comme un **retour** (poussé au moment où le joueur se demande « pourquoi ? »).

**M3. On lit des évènements, on ne vit pas une histoire.**
- Le matériau est riche et excellent :
  - 208 évènements en prose sourcée, dont 138 historiques et 153 illustrés ;
  - 253 portraits historiques ;
  - 3 actes (ADR 0784) ;
  - des jalons datés : Écluse, Crécy, peste, Calais, Poitiers, Brétigny…
- Ce qui l'abîme :
  - Les décisions arrivent en rafale : 3 bloquantes d'un coup au tour 3.
  - Le journal reste un registre sec (« {A} déclare la guerre à {B} ({motive}) »).
  - Les lettres réemploient ces lignes sous une rubrique.
  - Les portraits n'apparaissent pas dans la Chronique.
  - Aucun personnage ne revient d'une saison à l'autre comme protagoniste : héritier, rival, favori, traître.
- Cause : la narration est un **catalogue d'évènements**, sans **voix** (chroniqueur), sans **fil** (arcs de personnages) et sans **rythme** (une décision forte par saison).

**M4. L'audio contredit l'identité « réaliste XIVᵉ ».**
- Anachronisme, assumé par l'ADR 0166 :
  - La campagne de France joue en premier deux pièces au luth de Tomsinska ; la guerre joue **une seule** fantaisie pour violes de Moulinié (XVIIᵉ).
  - Les replis piochent dans Kevin MacLeod (« médiéval générique »).
  - Pourtant des pièces d'Ars nova et de Machaut sont déjà sur le disque (menu, replis).
- Pas de ponctuation sonore à l'apparition d'une décision ou d'une déclaration de guerre : les sons d'évènement sont regroupés en fin de tour (`event_sfx`), et `alert.wav` n'est pas utilisé sur la carte.
- Pas de musique de changement d'acte, de victoire ni de défaite de campagne.
- La boucle d'ambiance totalise environ 5,5 min sur 9 fichiers.
- En bataille, le filtre passe-bas de la musique de base est calculé mais pas appliqué (`base_cutoff_hz_not_applied` dans le dump).

**M5. Accessibilité : un plancher typographique contourné, pas de sous-titres.**
- Environ 16 appels sont sous le corps Caption de 15 px, surtout dans le HUD de bataille : 10 px dans `cards.rs:504`, `commands.rs:324`, `formation_menu.rs:326`, 11 px dans `abilities.rs:338`, `cards.rs:459`.
- Pas d'option de sous-titres, alors qu'il y a 295 voix (conseiller, cris, discours).
- Un seul mode daltonien (booléen Okabe-Ito).
- `text_size` ne s'applique qu'aux panneaux construits ensuite.
- Le délai des bulles est fixe (0,5 s).
- Pas de police lisible de repli (seulement EB Garamond et IM Fell).
- Le rouge des villes ennemies n'a pas d'alternative de forme.

**M6. Trois langages d'icônes, deux familles de cadres.**
- Icônes :
  - miniatures générées en couleur (179 à l'écran) ;
  - icônes à l'encre (244) ;
  - médaillons (15) ;
  - environ **107 glyphes Unicode** (₶ ▲ ▼ ✓ ✗ ⚔ ★ ✠ ↑ ▶), rendus par la police Noto de repli, donc d'un autre trait.
- Cadres : le kit enluminé (`ui/illumination`, ADR 0050/0135) coexiste avec les plaques cuir et laiton (`ui/fa`, `FaPlate`). La bataille utilise le même kit, mais avec des tailles codées en dur.
- Résultat : l'interface est parchemin par ses cadres, mais tableau de bord par ses signes.

**M7. Vocabulaire et chiffres qui se contredisent.**
- Pour les troubles, l'interface dit tour à tour « mécontentement », « agitation » (récompenses de mission) et « troubles ». Pour les mêmes lieux, « localités », « colonies » et « chantiers libres ».
- Le choix de faction affiche le revenu brut, la barre du haut le solde net.
- Le fil met les guerres d'un beylik anatolien dans la rubrique « Guerre » du roi de France.
- `hud_labels.json` impose pourtant « un seul terme pour le mécontentement dans tout le jeu ».
- Cause : chaque lot rédige ses textes dans son propre fichier (`missions.json`, `rules/advisor.json`, `next_hints.json`, `transitions.json`) sans glossaire commun vérifié.

### Mineurs
- Menu principal : titres blancs sur parchemin clair et un cadre vide en haut à droite (`c1_menu`).
- La carte du choix de faction est un aplat sans noms de royaumes.
- Les textes coupés dans les cartes de recrutement (« (réserve… ») et dans les lignes du fil.
- Deux missions jumelles, générique et de faction, sont offertes ensemble (Ponthieu).
- `docs/wip/f8-tutorial.md` décrit encore le guide Godot ; `campaign/hud/mod.rs` dit « minimap non portée ».
- La rubrique du fil « Armées » montre l'étranger (« Saint-Empire romain fonde Compagnie… ») au même rang que ses propres armées.
- « Edimbourg » est cuit dans `menu_map.jpg` (relevé par la boucle 1).
- Le Prologue du menu est une suite de modales d'information. Il ne débouche pas sur la partie.
- Le Conseil existe en 2D et en 3D : deux portes pour la même fonction.

---

## (b) Refontes proposées

Principes tirés du genre, sans copier un titre :
- **budget d'attention** : 3 choses au plus réclament le joueur par tour ; le reste se consulte ;
- **le tour est une scène** : ouverture, décisions, résolution, récit ;
- **la cause est montrée au moment de l'effet**, pas rangée dans une base ;
- **apprendre en jouant un vrai scénario**, pas en visitant des boutons ;
- **une seule grammaire visuelle**.

| Principe | Ce que font les bons jeux du genre | Cent Ans aujourd'hui |
|---|---|---|
| Attention | un résumé d'ouverture de tour et une liste d'actions courte et triée ; le monde se consulte dans un registre | 9 canaux poussent en parallèle |
| Causalité | infobulles imbriquées sur chaque chiffre, avec l'écart et ses termes ; rapport de fin de tour qui nomme les causes | bulles imbriquées (ADR 0109) mais sans écart ; causes dans un 4ᵉ onglet, vides pour le joueur |
| Récit | des personnages récurrents, portraits dans les évènements, un narrateur, une décision forte à la fois | 208 bons évènements, livrés en catalogue et en rafale |
| Prise en main | une première partie courte avec but, qui débloque les systèmes un par un | visite de panneaux, contournable |
| Fenêtres | peu de grandes fenêtres à onglets, des fiches contextuelles | environ 22 fenêtres et 10 lanceurs |

### R1 — « L'Audience de saison » : un seul canal d'attention

**Vision.** Chaque saison s'ouvre sur une **Audience** : le conseil du roi, avec de vrais portraits, présente de 1 à 3 affaires. Ce sont les seules choses qui réclament le joueur. Tout le reste devient consultable : registre, lettres, journal. La cloche, le bandeau Conseiller, la carte « Que faire », les toasts d'information et la file d'avis fusionnent en un seul **Registre** à trois rangs :
1. **Affaires** : bloquant, 3 au plus.
2. **À surveiller** : 5 au plus.
3. **Nouvelles** : repliées, résumées.

```
┌─ Printemps 1338 — Audience ──────────────────────────────────────────┐
│ [portrait] Le chancelier Pierre Roger :                              │
│   « Sire, Édouard a débarqué en Flandre. »    [Voir] [Lever l'ost]    │
│ [portrait] Le trésorier :                                            │
│   « Le trésor baisse de 2 100 ₶ par saison :                          │
│     soldes (-1 800), Guyenne perdue (-600). »   [Pourquoi ?] [Impôt]   │
│ [portrait] Le chroniqueur : « Une décision vous attend :              │
│   Le raid de Cadzand. »                          [Trancher]           │
│ ── À surveiller (2) : Agenais mécontent 62 % · recherche oisive       │
│ ── Nouvelles du monde (19) : 3 lettres pour vous, 16 résumées    ▸    │
└──────────────────────────────────────────────────────────────────────┘
```

- **Règles.** Le budget est appliqué dans `game-api` (un nouveau module `agenda`) à partir des sources existantes : alertes, décisions, missions, conseil ADR 0789, offres. Le client ne fait qu'afficher.
  - Une décision de chronique bloquante passe en Affaire. Les autres attendent la saison suivante : **une décision forte par saison** au plus, sauf les crises.
- **Coût.** Moyen, environ 3 lots : agenda dans `game-api` avec tests de budget, interface du Registre, retrait des doublons.
- **Risques.**
  - Masquer une information utile.
  - Mitigation : le Registre garde tout, seule la *poussée* est limitée ; les seed bots vérifient qu'aucune décision n'expire sans être passée en Affaire.
- **ADR touchés** : 0752 (bandeau unique, absorbé), 0775 (avis ordonnés), 0281 (rapport de saison : remplacé par l'Audience et le Récit, voir R4), 0789 (conseiller de saison : devient une source de l'agenda), 0304 (offre de missions : passe en Affaire). Un nouvel ADR « budget d'attention ».

### R2 — « Printemps 1337 » : une première campagne-scénario

**Vision.** Le didacticiel devient une **première partie réelle et courte** (8 à 10 saisons) pour France, Angleterre ou Bourgogne. Elle a un but historique : par exemple, pour l'Angleterre, « tenir la Guyenne et prendre pied en Flandre avant l'hiver 1339 ». Chaque chapitre enseigne un arbitrage et explique pourquoi :

| Chapitre | Saisons | Ce qu'on apprend |
|---|---|---|
| 1. Le royaume | 1 | trésor, soldes, un bâtiment au bon endroit, avec une explication de son rendement |
| 2. L'ost | 2-3 | recruter, réserve d'hommes, ravitaillement, marche |
| 3. La première bataille | 4 | lancée depuis la campagne avec la couche de didacticiel de bataille (`battle_prologue`) intégrée ; rapport expliqué |
| 4. Le siège | 5-6 | engins, vivres, assaut ou blocus |
| 5. Le monde | 7-8 | une offre diplomatique, une décision de chronique, le score d'acte et ce qui le fait monter |

- **Ce qui change par rapport au guide actuel.**
  - Les étapes portent sur l'**état** (« l'ost a 800 hommes et des vivres pour 2 saisons »), pas sur un panneau ouvert.
  - Entrée ne termine pas une saison tant que l'objectif du chapitre n'est pas atteint ou explicitement passé. Le sceau dit « Le conseil attend : *lever l'ost* » et propose « passer ».
  - À la fin, le joueur continue la même partie.
- **Choix de faction.** Trois **maisons** en grand (contenu écrit), puis « Autres princes », marqués honnêtement « expérience générique : pas de missions propres ni de didacticiel ». On réduit la liste par défaut aux factions qui ont un intérêt de jeu : environ 12 proposées, les autres derrière « Toutes (expert) ».
- **Coût.**
  - Moyen-élevé : données de scénario, conditions d'état dans `game_api::progress`, intégration de la bataille de campagne.
  - Il réutilise TUTO (étapes recrutement, siège, bataille, ravitaillement et leurs conditions).
- **Risques.**
  - Rigidité : la partie se déroule autrement que prévu. Mitigation : les chapitres sont des objectifs, pas des scripts, avec un repli « passer ».
  - Collision avec TUTO : à faire **après** la fusion de TUTO, en convertissant ses étapes.
- **ADR touchés** : F8 (guide), ADR « missions en choix » 0304 (missions du scénario imposées), choix de faction (ADR menu3), 0784 (acte 1 : le scénario en est l'ouverture).

### R3 — « Les cinq livres » : réduire 22 fenêtres à 5 livres et 2 fiches

**Vision.** L'interface devient une bibliothèque de chancellerie. Chaque livre a des onglets ; les fiches contextuelles (province, armée) restent à part.

| Livre (touche) | Absorbe |
|---|---|
| **Royaume** (R) | Ma faction, Économie (aperçu, commerce, monnaie), Finances royales, Féodalité, budget en un seul tableau, avoirs |
| **Armées** (A) | Recrutement global, Mes unités (U), rançons, ravitaillement |
| **Cour** (C) | Personnages, Conseil (2D ; la salle 3D devient un *fond* du livre, pas une 2ᵉ porte), ordres de chevalerie, héritier |
| **Diplomatie** (D) | inchangé, avec la carte diplomatique |
| **Chronique** (H) | Registre (R1), Décisions, Lettres, Causes, Journal (le journal et le fil disparaissent de l'écran de carte : un seul résumé replié) |

- **Fiches de carte.** La fiche de province reçoit un **intendant** : file de chantiers par province (« bâtir le meilleur rendement », « fortifier », « laisser ») et une vue « Chantiers du royaume » (toutes les localités libres, triées par rendement, un clic par chantier). L'alerte « 37 localités » disparaît au profit d'une ligne d'agenda **seulement si** le trésor dort (ADR 0749).
- **Coût.** Élevé, en 3 ou 4 lots. On déplace surtout des vues existantes, qui sont des constructeurs purs, ce qui facilite le déplacement.
- **Risques.**
  - Casser les noms des scripts et des tests de fumée (`hud:open_*`). Mitigation : garder des alias de noms un cycle.
  - Les joueurs habitués aux raccourcis : faire la réassignation dans `keymap.rs`.
- **ADR touchés** : 0002 (un fichier par jalon : révisé pour l'interface), 0345, 0346, 0280, 0285 (panneau Commerce), 0753 (focus).

### R4 — « Pourquoi ? » partout : la cause montrée au moment de l'effet

**Vision.** Toute variation visible porte sa cause, à un survol ou un clic au plus.
- **Barre du haut.** Chaque chiffre affiche son écart depuis la saison passée (▲ +1 200), avec une infobulle en **trois causes principales** (déjà dans `income_breakdown` et `unrest_breakdown`) et un lien Alt vers le détail (ADR 0109).
- **Le Récit de saison** remplace le rapport « auto » :
  - toujours présent, mais en **un quart d'écran**, non modal, replié en un clic ;
  - trois paragraphes : *Ce qui a changé*, *Pourquoi* (les 3 causes de plus fort effet, tirées du journal des causes), *Ce qui vient* (la prévision existante).
- **Journal des causes peuplé à chaque tour.** Aujourd'hui `journal_len = 0` au tour 3 : les variations de trésor de plus de 10 %, de troubles de plus de 10 points et les changements d'attitude d'IA deviennent des causes.
- **Score d'acte** : afficher « +4 si vous prenez Calais » à côté des objectifs, et le gain de la saison écoulée.
- **Bataille** : le rapport de fin dit pourquoi on a gagné ou perdu (archers non protégés, charge sur pieux, moral rompu par le flanc), à partir des évènements de `sim-battle`.

```
Trésor 28 728 ₶  ▼ −2 140 depuis l'hiver
  ├ Soldes de l'ost d'Édouard      −1 800
  ├ Guyenne : revenus perdus (siège)  −600
  └ Laine vendue à Bruges            +260     [Alt : détail complet]
```

- **Coût.** Moyen : 2 lots (`game-api` + causes du cœur ; interface et textes).
- **Risques.** Le bruit des causes. Le seuil est en données (`data/ui/causes.fr.json` et un nouveau `data/rules/cause_thresholds.json`).
- **ADR touchés** : 0754 (causes), 0281 (rapport), 0109 (bulles), 0789.

### R5 — « La voix du chroniqueur » : passer du catalogue au récit

**Vision.**
- **Un chroniqueur par camp** écrit la saison en prose : Jean le Bel puis Froissart pour l'Angleterre et le Hainaut, les Grandes Chroniques pour la France, Monstrelet pour la Bourgogne, un « clerc de la cour » générique pour les autres. Les phrases sont des gabarits en données sur les causes de R4, avec des variantes par ton (victoire, deuil, disette).
- **Arcs de personnages** : 3 rôles suivis (héritier, rival, favori) apparaissent avec leur portrait dans l'Audience et les lettres. Leurs évènements se chaînent (la `kind: chained` existe déjà pour 8 évènements).
- **Changement d'acte** : planche illustrée, musique dédiée, bilan rédigé.
- **Coût.** Moyen : écrire les données est le gros du travail, le code est léger.
- **Risques.** La répétition des gabarits. Mitigation : un historique anti-répétition, et des seed bots qui comptent les phrases identiques.
- **ADR touchés** : 0784/0785 (actes), 0721 (évènements à fenêtre), nouvel ADR « voix du chroniqueur ».

### R6 — « Une seule grammaire sensible » : son, signes, lisibilité

- **Musique** (révision de l'ADR 0166) : campagne et guerre en répertoire des XIVᵉ et XVᵉ siècles : Machaut, Landini, Ars subtilior, Dufay, Binchois, chansons de l'Agincourt Carol, estampies. Plusieurs sont déjà sur le disque. Le Kevin MacLeod sort des listes primaires, et la guerre a au moins 4 pièces.
  - Des **stingers** de 2 à 4 s : décision, guerre déclarée, place prise, mort d'un roi, changement d'acte (`sound_bank.json`).
  - Les enregistrements libres restent à sourcer (Wikimedia Commons, Musopen ; vérifier la clause de partage à l'identique CC BY-SA, encore ouverte dans `a2-audio-credits.md`). Coût cloud nul.
- **Signes.** Remplacer les 107 glyphes Unicode par des icônes à l'encre de la même famille : ₶, ▲▼ et ✓✗ en icônes vectorielles cuites en PNG. Une seule famille de cadres : les plaques FA sont réservées aux sceaux et aux moments solennels, comme le prévoit déjà `fa_ui_assets.json`.
- **Accessibilité.**
  - Plancher de 13 px appliqué par un test : aucun `text_font(.., n < 13)`, et Caption de 15 px pour tout texte de lecture.
  - Sous-titres des voix.
  - Délai des bulles réglable.
  - Police de repli lisible (Atkinson Hyperlegible ou Noto Serif).
  - `text_size` appliqué à chaud.
  - Un motif sur les villes ennemies en mode daltonien.
- **Coût.** Moyen et morcelable : 3 petits lots indépendants.
- **ADR touchés** : 0166, 0247, 0060, 0065 (icônes à l'encre), 0050/0135 (kit), 0155 (rouge ennemi).

---

## (c) Ordre recommandé

1. **R4 « Pourquoi ? » et R1 « Audience »**, en même temps : même source de données (agenda et causes dans `game-api`), le plus fort effet sur la compréhension pour le coût le plus faible. R4 d'abord dans le cœur, R1 consomme ses causes.
2. **R6, sous-lot accessibilité et glyphes**, en parallèle : petit, indépendant, réduit la fatigue de lecture.
3. **R2 « Printemps 1337 »**, après la fusion de TUTO : il réutilise l'Audience pour s'adresser au joueur.
4. **R3 « Cinq livres »** : le plus coûteux. Il se fait mieux quand R1 a déjà retiré journal, fil et cloche de l'écran.
5. **R5 « Chroniqueur »** puis **R6 musique** : ils s'appuient sur les causes (R4) et sur l'Audience (R1).

---

## (d) Specs des lots prioritaires

Règles communes :
- toute règle (seuils, budget, tri) vit dans `core/` (`game-api` ou `sim-campaign`) avec ses tests ;
- textes et seuils dans `data/`, validés par `data/schemas/` ;
- le client ne fait qu'afficher ;
- chaque lot commence par son squelette (API, fichiers vides, tests ignorés) et sa note `docs/wip/<lot>.md`.

### Lot XP-POURQUOI (R4) — cause au moment de l'effet. Modèle : Opus ou Sonnet, 1 à 2 jours d'agent

**Cœur et API.**
1. `core/crates/sim-campaign/src/causes.rs` : 3 nouveaux `CauseKind`.
   - `TreasurySwing` : le trésor varie de plus de `treasury_pct` d'une saison à l'autre ; `subject` est la faction ; le poids vient de l'ampleur.
   - `UnrestSwing` : les troubles d'une province varient de plus de `unrest_points`.
   - `AttitudeShift` : l'attitude d'une IA envers le joueur change de palier (ADR 0789 : réutiliser la détection de posture).
   - Construction dans `cause_builders.rs`, à partir de `income_breakdown` et `unrest_breakdown` : les 3 termes de plus fort effet deviennent les `factors` de la cause.
2. `data/rules/cause_thresholds.json` (nouveau) et son schéma `data/schemas/cause_thresholds.schema.json` : `treasury_pct` (0,10), `unrest_points` (10), `max_per_season_player` (6). Textes dans `data/ui/causes.fr.json`.
3. `core/crates/game-api/src/causes.rs` : `season_story(state, data, faction) -> SeasonStory`.
   - `changed` : les écarts des chiffres de la barre du haut.
   - `why` : les 3 `top_causes` de la saison, poids décroissant.
   - `next` : la `forecast` existante.
   - `act_delta` : le score d'acte gagné, avec ses termes.
4. `game-api/src/hud.rs` : `TopBar` reçoit, pour chaque chiffre, `delta_since_last_season` et `top_terms: Vec<(label, amount)>` (3 au plus).

**Client.**
5. `client/src/campaign/hud/bar.rs` et `text.rs` : écart sous chaque chiffre (▲/▼ à l'encre, voir R6) et infobulle « 3 causes + Alt : détail ».
6. `client/src/campaign/hud/report.rs` : le mode `auto` devient **Récit** : panneau non modal en bas à gauche, trois blocs, repli en un clic ; `always` et `off` restent. Section de dump `season_story`.
7. Fil (`feed.rs`) : ne plus tronquer au milieu d'un nombre ; la ligne dit le fait, l'infobulle porte le détail.

**Tests.**
- `sim-campaign` : une saison avec une solde doublée produit un `TreasurySwing` dont le premier facteur est l'entretien ; une saison calme n'en produit aucun.
- `game-api` : `season_story` est déterministe à graine fixe (instantané JSON) ; `top_terms` somme à l'écart ±1 %.
- Client : test unitaire du texte de bulle (`bar.rs`) ; `client/tests/smoke/campaign_turns.txt` vérifie que `season_story.why` n'est pas vide au tour 3.
- Seed bots, passe rapide : nouvel invariant « au plus `max_per_season_player` causes joueur par saison ».

**Critères d'acceptation.**
- Au tour 3 anglais (graine 1337), le dump `season_story.why` contient au moins une cause.
- Toute variation de trésor de plus de 10 % a sa cause.
- Chaque chiffre de la barre du haut a un écart et 3 termes au plus.
- Aucune ligne du fil n'est coupée dans un nombre.
- Clippy, tests, fumée et seed bots rapides verts.

**ADR.** Nouvel ADR « La cause au moment de l'effet », qui amende 0281 (rapport `auto` devient Récit) et 0754.

### Lot XP-AUDIENCE (R1) — budget d'attention et Audience de saison. Opus, 2 à 3 jours, après XP-POURQUOI (cœur)

**Cœur et API.**
1. `core/crates/game-api/src/agenda.rs` (nouveau) : `agenda(state, data, faction) -> Agenda { affairs: Vec<Affair> (≤ 3), watch: Vec<Item> (≤ 5), news: NewsDigest }`.
   - **Sources** : alertes de la cloche (`hud`), décisions de chronique (`chronicle`), offres de mission (`missions`), conseil de saison (`advisor`), offres diplomatiques, sièges, armées ennemies.
   - **Tri** : bloquant puis menace sur une place du joueur, puis échéance, puis poids. Les règles de tri et les plafonds sont dans `data/rules/agenda.json` (nouveau, avec son schéma).
   - Chaque `Affair` porte un `speaker` : `chr_*` du conseil (`court`), rôle et portrait. Elle porte aussi `action` (cible d'interface : ouvrir un panneau, centrer, trancher) et `cause_id` (lien vers R4).
2. `sim-campaign/src/chronicle.rs` : **une décision de chronique non critique par saison**. Les suivantes sont différées d'une saison, sans expirer pendant le report. Le plafond va dans `data/rules/agenda.json` et la règle est testée.
3. Une offre de mission ne peut plus expirer sans être passée au moins une saison en Affaire. L'invariant va dans `sim-campaign/src/invariants.rs`, vérifié par les seed bots.

**Client.**
4. `client/src/campaign/hud/audience.rs` (nouveau) : carte d'Audience à l'ouverture de saison (non bloquante, se replie en Registre), portraits (`court/portrait.rs`), boutons d'action.
5. Retrait des doublons à l'écran de carte :
   - la carte `hint.rs` est absorbée ;
   - le bandeau Conseiller ne garde que les répliques vocales, sous-titrées ;
   - la cloche se réduit à un compteur qui ouvre le Registre ;
   - les toasts d'information vont dans « Nouvelles ».
   Les noms `hud:*` utilisés par les scripts de fumée sont gardés comme alias.
6. `client/src/campaign/hud/registry.rs` : Registre à trois rangs (Affaires, À surveiller, Nouvelles résumées par rubrique avec un compte). Le monde étranger est replié par défaut.

**Tests.**
- `game-api` : jamais plus de 3 Affaires (propriété sur 200 graines × 12 saisons) ; une décision bloquante est toujours une Affaire ; ordre déterministe.
- `sim-campaign` : deux décisions le même tour, la seconde passe à la saison suivante avec son échéance décalée.
- Seed bots : « aucune mission expirée sans avoir été Affaire ».
- Client : fumée `campaign_turns.txt`, dump `agenda` avec `affairs.len() <= 3` à chaque tour ; dump `campaign_hud_layout` sans chevauchement.

**Critères d'acceptation.**
- Au tour 3 anglais, 3 Affaires au plus sont poussées, contre 9 alertes, 3 décisions, un bandeau et une carte aujourd'hui.
- L'écran de carte n'a plus qu'un seul endroit qui « parle » au joueur.
- Le joueur peut finir la saison avec Entrée, sauf Affaire bloquante, et le sceau dit laquelle.
- Le dump `ui_kit.widgets.total` de la carte au repos baisse d'au moins 20 %.

**ADR.** Nouvel ADR « Budget d'attention et Audience ». Il révise 0752 (bandeau), 0775 (avis), 0789 (le conseiller devient source) et 0304 (offres).

### Lot XP-LISIBLE (R6, accessibilité et signes) — Sonnet, mécanique, 1 jour, parallèle

1. **Plancher typographique.**
   - `client/src/ui/kit/style.rs` : `MIN_TEXT_PX = 13.0`.
   - `theme.text_font(.., n)` borne à ce plancher, ou la fonction est retirée au profit de `UiType`.
   - Corriger les environ 16 appels sous 15 px (`battle/hud/cards.rs`, `commands.rs`, `formation_menu.rs`, `abilities.rs`, `speech.rs`, `result.rs`, `journal.rs`, `ui/kit/tooltip.rs`).
   - Test : un test de source (grep dans `client/tests/`) refuse un littéral sous 13.
2. **Glyphes vers icônes à l'encre.**
   - Ajouter à `game/assets/icons/ink/` : `livre`, `hausse`, `baisse`, `oui`, `non`, `epees`, `etoile`, `croix`, `fleche_droite`, `fleche_gauche`, dessinés à la main ou en SVG game-icons cuits en PNG, jamais en low poly.
   - Les déclarer dans `data/ui/icons_ink.json`.
   - Remplacer les glyphes des widgets (pas des textes de prose) par `icon_tinted`.
   - Garder `₶` dans les montants en ligne, avec une police qui l'a en EB Garamond, sinon l'icône.
   - Le dump `ui_kit.icons.unresolved_on_screen` doit rester vide.
3. **Réglages** (`client/src/settings/model.rs`) :
   - `subtitles` (bool, vrai par défaut), qui affiche le texte des voix du conseiller et des discours dans le bandeau et le HUD de bataille ;
   - `tooltip_delay` (0,2 à 1,5 s) ;
   - `readable_font` (bool : Noto Serif ou Atkinson Hyperlegible sous `game/assets/fonts/`, licence OFL) ;
   - `text_size` appliqué à chaud (reconstruction des panneaux ouverts sur `SettingsChanged`).
4. **Glossaire.**
   - `data/ui/glossary.json` (nouveau, avec son schéma) : terme canonique et termes interdits, par exemple « mécontentement » contre « agitation », « localité » contre « colonie » dans l'interface.
   - Test `uv run --project tools pytest` : il balaie `data/**/*.json` (champs de texte) et `client/src` (littéraux FR) et échoue sur un terme interdit.
   - Corriger les occurrences actuelles : `sim-campaign/src/missions.rs` (« agitation » dans le texte des récompenses), `data/rules/advisor.json` (« colonies »).
5. **Raccourcis.**
   - Lier K (Codex) à `keymap.rs`.
   - Donner une touche à Recrutement, Ma faction et Féodalité.
   - Retirer de F1 les raccourcis non câblés B, G et L, ou les câbler.

**Tests.**
- `cargo test --features dev` (plancher, réglages sérialisés).
- Fumée `campaign.txt` et `battle.txt` : dump `ui_kit.icons.unresolved_on_screen` vide.
- Dump `settings` avec les nouvelles clés.

**Acceptation.**
- Plus aucun texte sous 13 px.
- Plus aucun glyphe Unicode dans les widgets du HUD.
- Sous-titres présents pour toute voix jouée (`advisor_band.history` et `battle_speech` portent le texte).
- `layout.failures = 0` à 1280×800 et à `ui_size` 1,25.

**ADR.** Amende 0065 (icônes à l'encre partout) et 0753 (raccourcis).

### Après ces trois lots
R2 « Printemps 1337 » sera spécifié une fois TUTO fusionné :
- convertir les 18 étapes en 5 chapitres à conditions d'état (`game_api::progress::sim_objective_met` existe) ;
- brancher `battle_prologue` sur la première bataille de campagne (`flow/`) ;
- bloquer la fin de saison derrière l'objectif du chapitre, avec un bouton « passer » ;
- refondre le choix de faction en trois maisons, puis « Autres princes (générique) ».
