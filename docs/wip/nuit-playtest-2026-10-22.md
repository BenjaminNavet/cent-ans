# Nuit de playtest autonome — 22/10

Mandat joueur (22/10) : jouer comme un joueur avec une vraie fenêtre (`--visible-window`, muet), captures réelles, améliorer le jeu et l'expérience joueur toute la nuit, tout tester.
Axes : parcours complet joueur ; qualité visuelle / perf. Git : commits sur main, pas de push. Cloud ≤ 5 $ (consigné dans `docs/budget.md`). Focus autorisé, son coupé (`CENT_ANS_MUTE=1`).
Captures : budget du CLAUDE.md relevé pour la nuit (travail visuel demandé) ; stockées hors git (`out/`).

## État
- [x] Build release, fenêtre réelle, captures (tools/playtest_window.sh)
- [x] Parcours testé : menu, choix de faction, campagne (fin de saison + confirmation), bataille (IA vs IA accélérée, cinématique), Codex, Réglages, Crédits
- [ ] Écran de résultat de bataille, sauvegarde/chargement, tutoriel, siège : à tester

## Constats (réels, fenêtre visible)
- Le `shot` interne du swapchain est noir sur la campagne ; l'écran réel est correct (outil de capture seulement).
- Perf en fenêtre : ultra ≈ 61 ms/image (16 i/s), haute 16,7 ms (60 i/s, vsync) : cause principale des apparitions tardives.
  Ultra = MSAA 4x/2x carte, SDFGI, ombres 500 m, brume volumétrique, relief 6 px (render_quality.json). Réglage joueur laissé tel quel.
- Marqueurs de campagne : stables dès la 1re image (labels_visible 32) ; les arrivées tardives sont du rendu lent.
- Menu principal : texte blanc peu lisible sur la carte claire (à corriger).
- Bataille : régiments = rectangles rigides pivotant autour de leur centre (core), figurines collées aux places.

## Fait
- `client/src/battle/units/follow.rs` : suiveurs par figurine (vitesse/rotation limitées, cap = direction de marche), tests ; commit f73dada3d.
- Clic armée : le relâchement ne sélectionne plus aussi la province/ville sous l'armée (`ProvinceClickGuard.army_press`).

## Prochaine étape
1. Vérifier visuellement les suiveurs sur un pivot de régiment (gros plan) ; régler PULL/MAX_SPEED si besoin.
2. Collisions brouillonnes entre régiments = règles du cœur (separation.rs, push.rs) : lot à part avec la refonte mêlée (docs/wip/refonte-systemes.md § 2), tests b6 / Crécy / Poitiers à surveiller.
3. Contraste du menu principal ; écran de résultat ; sauvegarde/chargement ; tutoriel ; siège.

## Retours du joueur (22/10, pendant la nuit)
1. Campagne : éléments qui apparaissent en retard (étiquettes de villes par vagues après l'arrivée sur la carte — confirmé par captures `out/nuit/p8`).
2. Bataille : unités non organiques en mouvement (rectangulaires), rotation mauvaise, collisions brouillonnes.
3. Campagne : clics armée / ville confus quand les deux sont proches.
Sous-agents : haiku par défaut, sonnet pour le plus difficile (pas d'Opus).
Outil : `tools/playtest_window.sh script prefix secs interval` (fenêtre réelle ; le `shot` interne est noir sur la campagne, l'écran réel est correct).

## Souris injectée (22/10, commit 52ad4f2a8)
- Script client : `mouse x y`, `mouse_down|mouse_up <left|right|middle>`, `mouse_click x y [bouton]`, `mouse_drag <bouton> x1 y1 x2 y2 [pas]` (pixels logiques de la fenêtre). Pilote : `tools/playtest_window.sh`.
- Bug plein écran trouvé : `UiScale` = 1,76 (0,99 en fenêtre) ; le rectangle jaune de sélection (nœud d'UI) était placé en pixels fenêtre bruts, donc décalé de ×1,76 par rapport à la sélection réelle. Corrigé (`update_drag_rect`).
- Glisser droit (ligne de front) : aperçu des emplacements (rectangle + flèche de front par régiment, ligne tracée) ajouté ; l'ordre part avec largeur et orientation (déjà là). Reste à décider : sens exact de « clic gauche glissé » du joueur.
- Vitesse : le bridage x0,45 d'approche (ADR 0184) ne vaut plus que pour les camps IA (`ai_enabled`) ; marche du joueur 0,55 -> 1,32 m/s ; virage du joueur x2 (`player_turn_factor`). Tests de bataille du cœur verts ; échecs campagne (sauvegarde/diplomatie) viennent de la fusion m6 d'autres sessions.
- À tester ensuite : clic armée/ville à la souris sur la campagne, écran de résultat, sauvegarde/chargement.
