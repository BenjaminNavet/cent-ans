# Audits de recette condensés

Résumés des rapports de recette Q1 et Q5 (textes complets supprimés de `docs/audit/` ; consultables dans l'historique git
avec `git show 239b9f787:docs/audit/q1-recette.md` et `git show e9bfec0c8:docs/audit/q5-recette.md`).
Les audits `a1`-`a6`, `q3-recette` et `backlog-tw` restent dans `docs/audit/` (référencés par des ADR ou des notes de chantier).

## Q1 — recette et intégration (2026-09-25)

- **Objet** : partie jouée par un pilote envoyant de vraies entrées (`game/tests/q1_playtest.gd`) après les fusions de la nuit : menu, campagne,
  bataille, assaut de Bordeaux, panneaux, 16 fins de tour, sauvegarde, réglages, France 1920x1080 et 1280x720, Angleterre 1920x1080.
- **Corrigé (8 défauts)** : panique Rust de la « Retraite générale » (ordre typé `Array[int]` lu comme `VarArray`, c771ab84) ; plaques d'armées
  de la carte affichées par-dessus la bataille 3D (3d6a4d3d) ; bandeau de notification invisible sous les panneaux (c7fb18fb) ; bandeau de fin de tour
  citant des batailles étrangères (a499485c) ; menu « Son… » ouvrant aussi les Objectifs (53c3d970) ; panneau Objectifs étiré hors écran
  (a2fe2dc2) ; textes d'aide erronés (f1c7da4b) ; rapport de saison sans date (5c3089ba).
- **Restait** : édits et routes commerciales non fusionnés dans main (branche `integration/tw`) ; figurines d'armée géantes au zoom rapproché ;
  impossibilité d'ouvrir une province où stationne une armée ; écran de fin après retraite générale (« tient le champ ») ; performances en qualité
  Ultra (26 FPS en 1080p) ; tutoriel bloqué à l'étape 2 ; doublon des réglages de son ; solde négatif de l'Angleterre dès le tour 1.
  Traités ensuite par les chantiers UI3, T2, V3, EQ et SA (voir `docs/archive/chantiers.md` et `docs/wip/`).
- **Mesures** : menu vers carte 6 s ; fin de tour 250-430 ms ; chargement de bataille 3,2-3,9 s ; bataille 53-62 FPS ; carte Paris Basse/Haute/Ultra 46/40/26 FPS (1080p).
- **Non couvert** : agents, propositions diplomatiques, sièges 3D joués, Bourgogne.

## Q5 — « comme un joueur » (2026-09-26)

- **Objet** : pilote `game/tests/q3_playtest.gd` sur main f82a03a6 : France 1920x1080 (10 tours, toutes phases) puis Angleterre 1280x720 (8 tours) ;
  corrections sur la branche `fix/q5-recette`. Conclusion du joueur : une seule partie suffit.
- **Corrigé (8)** : chances d'assaut identiques entre la barre d'armée et la fenêtre d'assaut (`CampaignState::assault_win_chance`) ; posture « Siège »
  levée après la prise d'une place ; diplomatie qui se rouvrait à chaque tour (désormais seulement pour une proposition nouvelle, pastille « Proposition ») ;
  identifiants internes (`army_0012`) remplacés par `CampaignState::army_name` ; rapport de saison masqué par un panneau ouvert ensuite ; libellé des chances
  en résolution automatique ; nuées de météo limitées à la vue stratégique lointaine ; écart des lettrines.
- **Restait** : batailles jouées trop faciles (IA tactique, lot EQ) ; mort systématique du roi ennemi dès la première bataille (`sim-battle` `kill_general`,
  à arbitrer) ; rapport de saison posé par-dessus la diplomatie ; couche commerce du pilote.
- **Mesures** : carte au départ 47 FPS (1080p) ; Londres de près en 720p Basse/Haute/Ultra 61/47/27 FPS ; fins de tour 110-830 ms.
