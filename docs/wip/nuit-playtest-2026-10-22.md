# Nuit de playtest autonome — 22/10

Mandat joueur (22/10) : jouer comme un joueur avec une vraie fenêtre (`--visible-window`, muet), captures réelles, améliorer le jeu et l'expérience joueur toute la nuit, tout tester.
Axes : parcours complet joueur ; qualité visuelle / perf. Git : commits sur main, pas de push. Cloud ≤ 5 $ (consigné dans `docs/budget.md`). Focus autorisé, son coupé (`CENT_ANS_MUTE=1`).
Captures : budget du CLAUDE.md relevé pour la nuit (travail visuel demandé) ; stockées hors git (`out/`).

## État
- [ ] Build release du client
- [ ] Parcours : menu → nouvelle campagne → tours → bataille → résultat

## Constats
(à remplir)

## Prochaine étape
Lancer le parcours en fenêtre visible.

## Retours du joueur (22/10, pendant la nuit)
1. Campagne : éléments qui apparaissent en retard (étiquettes de villes par vagues après l'arrivée sur la carte — confirmé par captures `out/nuit/p8`).
2. Bataille : unités non organiques en mouvement (rectangulaires), rotation mauvaise, collisions brouillonnes.
3. Campagne : clics armée / ville confus quand les deux sont proches.
Sous-agents : haiku par défaut, sonnet pour le plus difficile (pas d'Opus).
Outil : `tools/playtest_window.sh script prefix secs interval` (fenêtre réelle ; le `shot` interne est noir sur la campagne, l'écran réel est correct).
