# SM1 — smoke de main réparé

État : les deux régressions sont corrigées ; smoke vert dans le worktree (28 « smoke OK », code 0).

## 1. Plantage « Message queue out of memory » (code 138, étape campagne)
- Bisect (smoke comme juge, bornes b84344d2 bon / 848119b5 mauvais) : premier commit fautif
  **6efd3c84** `wip(ui1): textured parchment theme…` (UI1, interface enluminée).
- Cause : `DiplomacyPanel._fit_minimap` (branché sur `MapHolder.resized`) fixait la taille
  minimale de la vue à `taille du support − 16 px`. Avec le thème enluminé, le cadre de la
  mini-carte (`CampaignMinimap`, StyleBoxTexture du kit) dépasse 16 px (≈ 24 px) : la taille
  minimale du support dépasse sa taille, il grandit, `resized` repart… boucle infinie de
  tri de conteneurs dans une seule vidange de la file (tailles > 60 000 px au plantage).
- Correction : soustraire le cadre réel (`minimap.combined_min − view.combined_min`) + 4 px.
  Point fixe stable, quel que soit le thème. Pas de changement de la taille de la file.
- Note : la session UI1 a poussé en parallèle une correction équivalente dans main
  (8dd69a75) ; à la fusion, sa version a été gardée (même principe, même jeu de 4 px).

## 2. « music playlist too short » (campaign, war, court)
- Cause : le smoke pointe `MapPaths.data_dir` vers `game/tests/fixtures` (carte seule) ;
  `audio/music.json` y manquait. Même défaut latent pour la banque sonore et les voix VO1.
- Correction : `SoundBank.data_path(relatif)` = dossier de données du jeu, puis `data/` du
  dépôt à défaut (même motif que `BattleStandards.read_data`, `FrontEndData.data`) ; utilisé
  par la banque sonore, les playlists (`AudioDirector`) et les répliques (`VoiceLines`).

## Méthode de diagnostic (réutilisable)
Sonde temporaire dans le smoke : connecter `resized` / `minimum_size_changed` /
`sort_children` de chaque Control ajouté (`node_added`) et imprimer chemin, taille et taille
minimale au 2000e appel : le nœud qui boucle apparaît aussitôt.

Prochaine étape : aucune (fusion dans main par l'orchestrateur).
