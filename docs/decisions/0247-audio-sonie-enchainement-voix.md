# 0247 — Audio : sonie des morceaux, enchaînement, listes fallback, langues de bataille

Date : 2026-10-09. Lot RX `audio` (revue d'experts, `docs/wip/rx/audio.md`). Complète les ADR 0154
(rotation mélangée) et 0166 (musique de campagne calme).

## Contexte

La revue audio relève : morceaux de sonie très inégale (de -35,6 à -11,7 dBFS en moyenne, `volume_db`
codé à 0), coupure sèche entre morceaux, chant de 9 s bouclé pendant toute une bataille, listes
`fallback` jamais jouées (83 Mo), une même pièce servant menu, guerre et bataille, crêtes à 0 dBFS,
raccords de boucle d'ambiance audibles, clic d'interface doublé, 169 factions sur 177 criant en
français, pas de réglage séparé de la musique de bataille.

## Décision

- **Sonie des morceaux** : `track_gain_db` (chemin → dB) dans `data/audio/music.json`, plus
  `loudness_target_lufs`. Calculé par `tools/cent_ans_tools/audio_mastering.py gains --apply`
  (EBU R128 via `ffmpeg ebur128`, cible = médiane des morceaux `primary`, soit -16 LUFS : le niveau
  global ne bouge pas, seuls les écarts sont résorbés ; gain borné à -12/+9 dB et à -1 dBTP de crête).
  `AudioDirector` applique le gain au lieu de `0.0` ; `BattleMusicDirector` l'ajoute à `base_db`.
- **Variantes d'un événement** : `file_gain_db` (variante → dB) dans `sound_bank.json`
  (`audio_mastering.py variants`) : les variantes à plus de 3 dB de la médiane du groupe sont
  ramenées à ± 3 dB, sans jamais dépasser -1 dBFS de crête (les variantes faibles à forte crête ne
  sont donc pas remontées : il faudrait les compresser). Lu par `SoundBank.last_gain_db`, ajouté au
  `volume_db` de l'événement ; les `volume_db`, priorités et cooldowns ne changent pas.
- **Enchaînement** : fondu enchaîné de `crossfade_seconds` (3 s, en données) quand le morceau entre
  dans sa dernière seconde-fondu (`should_crossfade`, morceaux de moins de 4 fondus exclus) ; plus de
  redémarrage à plein volume : chaque morceau monte depuis le silence jusqu'à son gain.
- **Bataille** : plus de boucle. `BattleMusicDirector` tire dans le sac mélangé et sauvegardé de
  `battle` (`AudioDirector.next_track("battle")`, jamais deux fois de suite) et enchaîne à la fin du
  morceau. `battle` = estampie (Robertsbridge) + 4 pièces martiales de Kevin MacLeod (≥ 90 s) ;
  seul `war.ogg` boucle, en dernier recours. La règle « Kevin MacLeod jamais en `primary` » (`test_era_music.py`) est levée pour `battle` seul : pas d'enregistrement d'époque martial de plus de 90 s. Le chant d'Azincourt (9 s) sort de toutes les listes de
  fond (fichier conservé, utilisable comme jingle).
- **Pièces exclusives** : menu, guerre et bataille n'ont aucune pièce commune, ni entre elles ni
  par fusion `war` + liste régionale (les pièces de menu/bataille sont retirées des listes de
  campagne). Test `rx_audio_test.gd`.
- **Listes `fallback` : intégrées à la rotation (choix b)**, pas retirées du dépôt. Après
  `fallback_every` (4) morceaux `primary` d'affilée dans un contexte, le tirage suivant vient du sac
  `fallback` ; un contexte sans `fallback` (battle) n'est pas concerné. Les 83 Mo restent donc
  utiles, la sonie étant désormais homogène. `fallback_every: 0` rétablit l'ancien « secours seul ».
- **Finitions de fichiers** (`audio_mastering.py limit|loopfade`, ffmpeg) : 17 échantillons à 0 dBFS
  re-encodés sous limiteur (crête ≤ -0,9 dBFS) ; 7 ambiances/nappes (countryside, forest, sea,
  wind_strong, clamor_bed, march_bed, melee_bed_3) refaites avec un fondu de boucle interne de 1,5 s
  (durée - 1,5 s). Ce ffmpeg n'a pas libvorbis : encodeur Vorbis natif (stéréo, 128 k), mono
  dupliqué sans perte de niveau. À rejouer si le pipeline de génération (`audio_bank`) régénère ces
  fichiers.
- **Clic d'interface** : un seul chemin, l'événement `ui_click` de la banque (`UiSounds`, avec
  recharge et variation de hauteur) ; `AudioDirector._on_button_pressed` n'appelle plus
  `sfx/ui_click.ogg` (fichier conservé, plus joué).
- **Langues de réplique** : `faction_language` couvre 112 factions sur 177 (`barks_languages.py`,
  table `CULTURE_LANGUAGE`) avec la langue existante la plus proche : romane → gascon (`oc`, 40),
  germanique → flamand (`nl`, 42), celtique → gallois (`cy`), anglais, écossais. Les 65 factions (25 cultures)
  sans langue proche (slave, turcique, arabe, grec, caucasienne, hongroise, baltique,
  finno-ougrienne, albanaise) restent en français : **besoin** d'enregistrements es, it, de, puis
  grec, arabe, russe/slave, turc (hors enveloppe ElevenLabs de ce lot, non dépensée).
- **Curseur** : bus `BatailleMusique` (enfant de `Musique`) ajouté à `PLAYER_BUSES`
  (« Musique de bataille », défaut 1,0) ; `Ambiance` avait déjà son curseur.

## Conséquences

- Niveaux homogènes : plus de morceau 6 à 9 dB plus fort à la rotation ; le défaut `Musique` 0,6
  reste à valider à l'oreille (aucun réglage de mixage changé sans écoute).
- Hors lot, proposé : conversion mp3 → Ogg Vorbis q4 des 72 morceaux (≈ -50 % de poids, boucle sans
  trou) — non faite (conversion en masse), à décider après écoute.
- Le test `rx_audio_test.gd` verrouille : gains présents, pièces exclusives, durées ≥ 60 s, fondu,
  langues des factions, gains de variantes, bus.
