# Recette Q5 — « comme un joueur » (2026-09-26)

Pilote `game/tests/q3_playtest.gd` (vrais clics, molette, touches, captures) sur main f82a03a6 :
France 1920×1080 (10 tours, toutes phases) puis Angleterre 1280×720 (8 tours). Corrections sur la
branche `fix/q5-recette`. (Le joueur a précisé ensuite qu'une seule partie suffit.)

## Corrigé

1. **Assaut : deux chances différentes.** La barre d'armée annonçait « Donner l'assaut
   (chances ≈ 50 %) », la fenêtre d'assaut qui s'ouvre ensuite « Victoire presque certaine,
   100 % » (Bordeaux ; Rouen : 52 % contre 100 %). La barre utilisait un simple rapport de
   puissances. Désormais `CampaignState::assault_win_chance` calcule la chance exactement comme
   l'avant-bataille (`battle_forecast`, refactorisé en `forecast_request`). L'IA garde
   `assault_odds` (ses seuils ne changent pas).
2. **« Siège » collé à l'armée après la prise.** `capture` levait le siège de la place mais
   laissait les assiégeants en posture `Siege` : l'armée affichait « 495 siège » et des tentes
   même en marche ailleurs. Les assiégeants présents repassent en posture normale.
3. **La diplomatie se rouvrait à chaque tour.** Dès que l'IA propose un accord commercial (presque
   chaque tour), la fenêtre plein écran s'ouvrait en fin de tour et masquait la carte (tours 2 à 9
   de la partie France). Elle ne s'ouvre plus que pour une proposition **nouvelle** ; tant qu'une
   offre attend, une pastille « Proposition » (cloche) ouvre la diplomatie.
4. **Identifiants internes dans le journal** : « La flotte de l'armée army_0012 rentre au port… »,
   « Débandade : l'armée army_0012… », disette, débarquement, blessés soignés, anéantissement,
   bataille navale. Nouveau `CampaignState::army_name` : « l'ost de Philippe VI de Valois »,
   sinon « l'ost de France » (11 messages).
5. **Le rapport de saison restait par-dessus la fiche de colonie** (liste de recrutement cachée,
   1080p et 720p). Il se referme dès que le joueur ouvre un panneau après lui.
6. **« Défaite presque certaine, 3 % » puis victoire nette en 2 min 36.** Les chances affichées sont
   celles de la résolution automatique ; le libellé le dit désormais (« En résolution automatique :
   3 % de chances … une bataille menée peut renverser l'issue »).
7. **Nuées sur l'Île-de-France dès la vue de départ** (Paris, Orléans, Troyes illisibles). Les
   nuées de la météo n'apparaissent plus qu'en vue stratégique lointaine (260-520 u., 60 %).
8. **Lettrines lues « D iplomatie »** (déjà vu en Q3) : écart ramené de 5 à 1 px.

## Restant (non corrigé ici)

- **P1 — Les batailles jouées sont trop faciles.** Les deux batailles de campagne, annoncées à 3 %
  et 0 %, ont été gagnées en 2 min 36 et 2 min 24 avec un seul ordre (« tout le monde attaque ») :
  18 % contre 40 % de pertes, puis 6 % contre 35 %. L'IA tactique ne tire pas parti de sa
  supériorité. Lot EQ (équilibrage, EQ6/EQ7 en cours) / IA de bataille.
- **P1 — Le roi ennemi meurt à la première bataille** dans les deux parties (Édouard III en 1337,
  Philippe VI en 1337), y compris quand son régiment n'est « qu'en déroute » (34/60 pertes).
  Risque par coup : pertes du tick / effectif × 0,4 (≈ 30 % pour la moitié des effectifs), mort
  certaine si le régiment est anéanti ; le chef charge en tête. Historiquement rare (Jean II est
  pris, pas tué). Lot EQ : `sim-battle/src/sim.rs` (`kill_general`), à arbitrer avec le joueur.
- **P2** — Quand la fin de tour ouvre à la fois la diplomatie (offre nouvelle) et le rapport de
  saison, le rapport se pose encore par-dessus la diplomatie.
- **P3** — Pilote : la couche commerce est passée de X à V (MF1) ; `phase_trade` presse encore X.
- Mesures : carte au départ 47 FPS (1080p) ; Londres de près en 720p : Basse 61, Haute 47,
  Ultra 27 FPS (Haute s'est nettement améliorée depuis Q3 : 24 → 47). Fins de tour 110-830 ms.
  Voix audibles (pic −12,9 dB) en bataille, siège et naval.

## Ce qui marche bien

Menu, choix de faction, chargement (6,4 s), avant-bataille illustrée, discours, bataille et
siège 3D avec échelles, incendies et journal, écran de fin détaillé, rapport de saison,
recrutement, édits, négociation avec contre-offre (DP2), agents, décisions de chronique,
sauvegarde / chargement rapides, filtres de carte (N, R, M), zoom jusqu'à 2,6 u. sur Londres.
