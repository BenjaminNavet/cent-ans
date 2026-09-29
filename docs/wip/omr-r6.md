# OMR R6 — musique orthodoxe/islamique + portraits (branche feat/omr-r6)

## État : terminé
- Partie A : `era_music.py` étendu (champ `max_seconds`, pause anti-limite de débit de l'API
  Commons, correction du fichier temporaire ffmpeg `.part.mp3`). Contextes `campaign_orthodox`
  (5 pistes : 2 hymnes byzantins, 3 chants znamenny, CC0) et `campaign_islamic` (5 pistes :
  enregistrements de 1929-1939, domaine public) ; ~24 Mo ajoutés ; toutes les cultures de
  factions ont une région ; schéma étendu ; tests étendus. Crédits : `CREDITS.md`, `SOURCE.md`.
- Écartés : « Rubba laylin » (nuba de Fès, CC BY 4.0) et oud d'Andy R. Jordan (CC BY-SA 3.0) :
  téléchargement bloqué par la limite de débit d'upload.wikimedia.org (réessayable en les
  rajoutant au manifeste). Steppe / nordique : aucune source libre convenable (groupe moderne
  tatar, extraits de quelques secondes) : pas de contexte créé ; cultures rattachées à
  orthodox / islamic / burgundy.
- Partie B : portraits d'Andronic III (base + âgé, stemma à pendeloques et loros) et d'Abu
  l-Hasan (base + âgé, sans auréole) régénérés (4 × gpt-5-image-mini, 0,18 $ réel), vérifiés à
  la lecture des images ; consignés dans `docs/budget.md` (cumul OM 8,25 $).
