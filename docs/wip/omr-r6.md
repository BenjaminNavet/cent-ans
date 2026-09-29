# OMR R6 — musique orthodoxe/islamique + portraits (branche feat/omr-r6)

## État
- Partie A : `era_music.py` étendu (champ `max_seconds`, pause anti-limite de débit API Commons,
  correction du nom du fichier temporaire ffmpeg `.part.mp3`). Contextes `campaign_orthodox`
  (5 pistes) et `campaign_islamic` (5 pistes) ajoutés ; toutes les cultures de factions ont une
  région (`culture_regions`), schéma étendu. Tests étendus (`tools/tests/test_era_music.py`).
- Reste : 2 pistes islamiques (Rubba laylin, oud) en attente (limite de débit upload.wikimedia.org),
  `SOURCE.md` (réécrit par le script quand tout est téléchargé), `CREDITS.md`, partie B (portraits).
- Steppe / nordique : aucune source libre convenable trouvée sur Commons (seulement un groupe
  moderne tatar, extraits de 40 ko) : non créés, cultures rattachées à orthodox/islamic/burgundy.
