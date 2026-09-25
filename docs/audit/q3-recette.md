# Q3 — recette « comme un joueur » (25/09, après une dizaine de sessions fusionnées)

Base : main 9fe0550a (VO1, UI1, AR1, musique, ZG, UX1/UX2, DP1, C4/C5, EQ1/EQ2, NV1/NV2,
SG1-SG3, PF1, DF1). **MF1 (menu Filtres) n'était pas dans main** (branche `feat/map-modes`) et
**EP (batailles épiques) n'y est arrivé qu'à la fin de la recette** (dc2360c6, fusionné avant le
smoke final, mais les batailles de cette recette ont été jouées sans EP).

Pilote : `game/tests/q3_playtest.gd` (dérivé de Q1). Vraies entrées poussées dans la fenêtre
(clics, Maj-clic, clic droit, molette, touches), onglets cliqués, captures, mesure des voix sur
le bus « Voix » (pic > -50 dB), ordres d'attaque en bataille (sélection des cartes d'unité, clic
droit sur l'ennemi le plus proche). `user://settings.cfg` sauvegardé avant chaque partie et
restauré après.

```
godot --path game --script res://tests/q3_playtest.gd -- --out=<dossier> \
  --faction=fac_france --turns=12 --res=1920x1080 \
  --phases=battle,siege,naval,actions,edicts,diplomacy,trade,agent,zoom,panels,turns,save,settings
```

Parties jouées :
- **France 1920×1080** : 12 tours (printemps 1337 → été 1340) avec recrutements réguliers,
  construction, édit, deux traités (Aragon : accord commercial ; Angleterre : trêve), commerce,
  espion recruté, déplacé et mis en mission, tous les panneaux, F5/F9, sauvegarde/chargement par
  le menu ; seconde partie : bataille depuis la carte, siège, zoom, réglages, qualité.
- **Angleterre 1280×720** : bataille avec ordres, siège de Rouen **joué en 3D (assaut)**, bataille
  navale (pas de Calais, 3 nefs contre 12), diplomatie, édit, recrutement, construction, agent,
  zoom ZG jusqu'à 0,3 unité, 6 tours, F5/F9, réglages.
- Naval complet en 1920×1080 (192 s, jusqu'à l'écran de fin).

Captures : `docs/audit/captures/q3/` (préfixes `fr`, `fr2`, `en`, `en2`-`en4`, `nv`, `ag`).

## Corrigé (un commit chacun)

| # | Défaut | Lot | Commit |
|---|---|---|---|
| C1 | **Bouton « Donner l'assaut » hors écran** : l'étiquette d'état du siège, à retour à la ligne sans largeur, mesurait ~1 800 px de haut au premier affichage ; le panneau d'actions d'ost montait jusqu'à y = -1 135 et le joueur ne voyait qu'un grand rectangle de vélin vide (capture `fr-033`). Aucun assaut possible à la souris en 1080p. | M8 / F10b | 85e8efd9 |
| C2 | **Touche R** : `map_toggle_trade` et `map_mode_religion` sur la même touche, la diplomatie la consomme d'abord : la couche commerce n'était jamais basculée au clavier (l'infobulle annonçait « (R) »). Commerce déplacé sur X. | C5 | 544aec3c |
| C3 | **Fenêtre de chronique plus haute que l'écran** (720p et 1080p) : titre à retour à la ligne sans largeur (une lettre par ligne au premier calcul), anciennes options encore comptées ; titre, illustration et bouton « Plus tard » hors champ (`en2-062` → `en4-010`). | M10 / AR1 | 7cd65af0 |
| C4 | **Rapport de saison hors écran en 720p** : avec la vignette AR1 (170 px), « Continuer » passait sous le bas de la fenêtre (y = 729 pour 720) ; la liste se réduit maintenant à la place disponible. | F3 / AR1 | 1b4dc2fe |
| C5 | Bandeau « La garnison ouvre ses portes et fait une sortie ! » dessiné **par-dessus le tableau de l'écran de fin de siège** (`en-031`). | UB1 / SG | bb1c54ac |
| C6 | **Barre d'agent** : les boutons d'action libérés restaient comptés dans la largeur jusqu'à la fin de l'image ; la barre faisait ~1 300 px et, ancrée au centre, sortait à droite (« Renvoyer » et ✕ hors écran en 720p, `en2-041`). Largeur juste et barre centrée (`ag-012`). | C6 agents | 6e28f35e |
| C7 | Écran de fin **naval** : texte posé sur le cadre de lierre (« Hommes », « Tués » amputés de leur initiale, `nv-014`) — marge 22 px pour un cadre de 38 px. | UI1 / NV | 74d24f03 |

## Restant, par priorité

### P1 — à traiter vite

1. **La bulle du conseiller (VO1) bloque les clics et masque les fenêtres modales.** Calque 40,
   `MOUSE_FILTER_STOP`, 440 px au centre bas : elle apparaît justement quand une fenêtre s'ouvre
   (début de partie, première bataille, première défaite, premier assaut) et se pose dessus.
   Constaté : le clic sur « Recruter » de Paris part dans la bulle (partie France : **0 unité
   recrutée au tour 1**) ; « Continuer » du rapport de saison touché en 720p ; bulle sur la liste
   des régiments de l'avant-bataille (`fr2-006`), sur le discours du roi (deux textes superposés,
   `fr2-009`), sur le tableau de fin de bataille (`fr2-017`), sur la liste des navires
   (`nv-006`), sur la diplomatie (`en2-015`). Le premier clic d'un joueur ne fait que faire taire
   le conseiller. Lot : VO1 (Q2 l'avait déplacée en bas au centre). Piste : ne capter que la
   souris sur un petit ✕, la placer hors des zones modales, ou la différer tant qu'un dialogue
   modal est ouvert.
2. **Zoom ZG au plus près : terrain nu et sans ville.** Avec la pyramide (lien vers le cache du
   dépôt principal), la caméra descend à 0,3 unité au-dessus de Londres, mais en dessous de ~3
   unités on ne voit plus qu'une plaine beige ondulée avec des cuvettes, sans texture, sans
   maison ni route, et le nom de la ville disparaît (`en3-014`, `en3-015`). À 20 unités, un
   **ruban rouge et gris géant** (≈ 1 km de large à l'échelle) traverse Londres (`en3-011`) :
   probablement une route commerciale ou un tracé non mis à l'échelle au palier proche. Sans le
   cache (clone neuf, worktree), le zoom s'arrête à 7 unités : c'est ce que verra tout joueur
   qui n'a pas lancé `cent-ans geo pyramid` (2,6 Go). Lot : ZG (ZG4-ZG6) ; ruban : C5
   `trade_route_layer` ou `road_renderer` à vérifier.
3. **Bataille navale : écran d'avant-bataille avec le mauvais camp (à confirmer).** Joueur
   France, flotte interceptée en Manche (mise en scène `debug_stage_naval`) : la colonne de
   gauche (« vous ») montre l'Angleterre, 5 nefs, amiral Édouard III, verdict **« Victoire
   presque certaine, 100 % »** ; la bataille se solde par « Défaite sur mer », 475 tués et 255
   prisonniers français (`nv-006`, `nv-014`). Soit `player_side` est inversé quand le joueur
   est l'intercepteur, soit la mise en scène est trompeuse. Lot : NV1/NV2
   (`naval_pre_battle_dialog.gd`, `get_pending_naval_battles.player_side`).
4. **Qualité « Haute » aussi lourde qu'« Ultra » en 1080p** : vue de Paris à 40 unités,
   Basse 58 FPS (1,5 M primitives), Ultra 22 FPS (16 M), **Haute 24 FPS (15 M)**. En 720p (Londres)
   61 / 35 / 58. Le préréglage par défaut d'une machine M4 Pro tombe sous 30 FPS dès qu'on
   s'approche d'une ville emblématique. Carte au démarrage : 42-49 FPS en 1080p. Lot : PF1 / PB1.
5. **Menu Filtres (MF1) absent de main** : impossible à recetter (branche `feat/map-modes` non
   fusionnée) ; les modes restent N / R / M et les quatre boutons de la minicarte. Lot : MF1.

### P2 — gênant

6. **Cartes d'alerte et infobulles de la carte actives sous les fenêtres modales** : en 1080p,
   le premier clic sur « Combattre » (naval) a été capté par la bulle « Aucune recherche en
   cours » de la pastille d'alerte, ouverte par-dessus le bouton (`nv-006`). Il a fallu cliquer
   deux fois. Lot : U5 / UX1 (alertes) — les pastilles devraient ignorer la souris sous un
   dialogue modal.
7. **La carte « Que faire maintenant » (UX2) recouvre le registre des agents** en 720p : le clic
   sur la ligne de l'espion part dans `NextHintCard`, l'agent n'est pas sélectionné. Lot : UX2.
8. **Barre d'agent : Échap ouvre le menu pause au lieu de la fermer** ; elle reste affichée
   par-dessus le panneau de faction, la cour et les techniques (`fr-054`, `en2-041`)
   jusqu'à un clic sur ✕. Lot : C6 agents (`_unhandled_input` après le menu pause).
9. **Siège 3D en 720p : brouillard épais et « Temps clair »** : l'en-tête annonce « Temps clair »
   alors que les murailles de Rouen ne sont que des silhouettes dans un brouillard blanc
   (fumée des faubourgs incendiés ?) ; au déploiement, la caméra cadre un bélier sur une plaine
   vide (`en-024`, `en-029`). On ne voit pas l'assaut. Lot : SG3 / V3.
10. **Bataille navale : caméra et lisibilité** : en 720p les navires anglais restent au bord
    inférieur, 80 % de l'écran est de la mer brumeuse (`en-041`) ; les boutons d'ordre sont gris
    sans explication visible tant qu'aucun navire n'est choisi (la consigne est une ligne
    grise). Bouton « Retour à la campagne » sans style sur l'écran de fin. Lot : NV2.
11. **Diplomatie : « Que faudrait-il ? » contredit la jauge** : Aragon à 73 % (« accepterait »)
    pour un accord commercial, le bouton répond « Rien de ce que vous pouvez offrir ne
    suffirait » ; le traité est ensuite signé. Le bandeau « Le traité est signé. » reste
    affiché en tête quand on passe à la France qui refuse. Lot : DP1.
12. **Bataille sans ordres : personne n'engage.** France contre Angleterre en 1080p, joueur
    passif : après 7 min 14 s de temps de jeu, 0 perte des deux côtés ; les archers anglais
    plantent leurs pieux et attendent (`fr2-013`). Normal pour un défenseur, mais la bataille
    ne se termine jamais d'elle-même. Avec des ordres (720p), la mêlée a lieu (≈ 230 pertes de
    chaque côté en 5 min) mais n'est pas finie à 7 min 23 s. Lot : B3 / R2b (IA), à surveiller
    avec EP.
13. **Lettrines UI1 lues comme deux mots** : « D iplomatie », « F rance », « C our »,
    « T echnologies » — l'initiale est dans une case séparée par un blanc et décalée
    verticalement (`en2-015`, `fr-054`, `en2-041`). Lot : UI1 (`lettrine.gd`,
    `illuminated_title.gd`).

### P3 — finitions

14. Le panneau de colonie : la liste de recrutement est sous le pli (il faut défiler pour voir
    autre chose que la bombarde verrouillée, `en2-021`). Lot : C5.
15. Chevauchement mineur : la pastille « Aucune recherche » et le nom de province du bas
    d'écran passent sous le rapport de saison et la diplomatie en 720p (`en2-055`), et le
    rapport de saison s'ouvre par-dessus le panneau de diplomatie resté ouvert.
16. « 18 ObjectDB instances were leaked at exit » (Angleterre 720p). Sans effet en jeu.

## Ce qui marche

- **Aucune erreur de script du jeu** sur toutes les parties (les seules erreurs venaient du
  pilote), aucune panique Rust. 12 tours France + 6 tours Angleterre sans blocage.
- **Voix** : elles se déclenchent et s'entendent (pic -12,9 dB sur le bus Voix) : conseiller au
  premier tour (`adv_start_france`, `adv_start_england`), première bataille, première défaite,
  premier assaut, province prise ; discours du chef avant bataille et siège ; répliques
  d'unités en bataille (928 à 1 679 images actives par bataille) et en naval. Cases « Conseiller »
  et « Répliques des unités » présentes dans Réglages → Son, curseurs thémés et cliquables.
- **Siège joué en 3D de bout en bout** depuis le bouton du bandeau : discours, échelles, bélier,
  incendie des faubourgs (13 maisons en feu), sortie de la garnison, écran de fin détaillé.
- **Bataille navale** jouable jusqu'au bout : abordages, prises, fuites, chronique, écran de fin.
- **Diplomatie DP1** : négociation par clauses, jauge avec considérations chiffrées, contre-
  proposition, refus motivé ; accord commercial signé ; routes commerciales visibles (12 routes,
  5-6 pour le joueur) et coupées par la guerre dans le rapport de saison.
- **Édits** (« Aide féodale » accepté), **construction**, **recrutement** (3 unités à Londres),
  **espion** recruté, déplacé au clic droit et mission « Renseigner » réussie avec rapport.
- **F5/F9** : la date revient bien à la sauvegarde ; fenêtres Sauvegarder / Charger du menu.
- Fins de tour **330-610 ms** ; menu → carte **6,7-8,0 s** ; bataille chargée en 3,6-4,4 s,
  siège 2,8 s, naval 2,4-2,8 s.

## Ce qui impressionne

- Le **style enluminé** est cohérent et beau : avant-bataille avec miniature, portraits et
  cartes d'unité peintes, écran de défaite sur fond de manuscrit, écran de chargement naval
  avec citation de Froissart, chronique illustrée, rapport de saison illustré (`fr2-006`,
  `fr2-017`, `en-035`, `en4-010`).
- **Paris à 7 unités** : Seine, île de la Cité, Notre-Dame, enceinte, milliers de maisons, pluie
  (`fr2-030`). Londres à 8 unités avec la Tamise et le pont (`en3-012`).
- Batailles : lignes de cavalerie en ordre, bannières, rivière et gués, journal de bataille
  riche ; flottes aux voiles armoriées et sillages.

## Ce qui déçoit (par rapport à un Total War)

- **L'interface se marche dessus** : bulle du conseiller, alertes, carte « que faire », barre
  d'agent et bandeaux se superposent aux fenêtres modales ; un joueur perd ses premiers clics.
  Total War garde une règle stricte : une fenêtre modale à la fois, le conseiller dans un coin.
- **Batailles lentes à se nouer et sans fin nette** : à ×4 il faut plus de 7 min de temps de
  jeu sans décision ; aucune bataille de la recette ne s'est finie seule (retraite générale à
  chaque fois). Les unités vues de loin sont de petites taches (`fr2-013`, `en-013`).
- **Le zoom extrême n'apporte rien** : sous 3 unités, un sol nu ; la ville disparaît.
- **Le siège 3D ne se voit pas** : brouillard, caméra mal placée.
- **Performance** : le préréglage Haute sous 30 FPS près de Paris en 1080p sur M4 Pro.

## Mesures

| Mesure | 1920×1080 | 1280×720 |
|---|---|---|
| Menu → carte prête | 7,7-8,1 s | 6,7-7,4 s |
| Fin de tour | 400-590 ms | 330-610 ms |
| Chargement bataille / siège / naval | 4,4 s / — / 2,8 s | 3,6 s / 2,8 s / 2,4 s |
| FPS carte au démarrage | 42-49 | 60 |
| FPS bataille (8 contre 6 régiments) | — | 48-61 |
| FPS siège, naval | — | 60 / 60 |
| FPS vue Paris (1080p) / Londres (720p), Basse / Ultra / Haute | 58 / 22 / 24 | 61 / 35 / 58 |
| FPS zoom au plus près (Paris 7 u. / Londres 0,3 u.) | 31 (10,9 M primitives) | 62 |

## Non couvert

- Menu Filtres (MF1 hors de main), batailles épiques EP (fusionnées après les parties).
- Bourgogne. Assaut sur brèche (murailles intactes à chaque fois). Sauvegarde puis chargement
  depuis le menu principal (seulement F5/F9 et fenêtres du menu pause).
- Bataille navale en commandant les navires (le pilote n'a donné aucun ordre naval).
