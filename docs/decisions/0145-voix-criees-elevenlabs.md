# 0145 — Voix de combat criées : ElevenLabs v3 et cris de guerre en chœur

## Contexte
Retour joueur (30/09) : voix « molle » en combat, « Montjoie ! Saint-Denis ! » en tête.
Mesure : les cris VO1 (gpt-audio-mini) sortent à ~130 Hz médians, la hauteur de la parole
calme (« Monseigneur ? » : 114 Hz) ; un cri d'homme monte à 200-350 Hz. Le modèle conversationnel
lit le texte, il ne crie pas, quelle que soit la consigne.

## Décision
- Les situations criées des répliques (attaque, charge, déroute, victoire, chute du général) et les
  cris de guerre passent par **ElevenLabs v3 via fal.ai** (0,10 $ / 1000 caractères), avec des
  balises d'émotion par situation (`shout_tag` dans `data/voice/barks.json`). Sélection et marche
  restent en gpt-audio-mini : ElevenLabs y garde un léger accent anglais.
- Chaque prise est contrôlée localement (whisper : mots ou lettres sans diacritiques, langue
  attendue ou tolérée ; F0 médian ≥ 180 Hz ; durée), jusqu'à 2 prises par voix puis les autres voix
  de la langue ; en échec, l'ancien clip reste.
- **Cri de guerre = chœur** (`cry_chorus` dans `speech_voices.json`) : voix de tête pleine, 4 voix
  recalées sur sa durée, 2 couches chacune (hauteur ±4 %, retard ≤ 60 ms, gain 0,25-0,4), écho de
  plein air. Un premier essai sans recalage était inintelligible (whisper : « Bonsoir ! Salut ! »).
- Traitement combat : passe-haut, compression, présence 2,6 kHz, −13 LUFS (parole −16).
- En bataille, la première charge de chaque camp lance le chœur de sa faction (spatialisé, 700 m),
  à la place de la réplique de charge.

## Conséquences
- 69 répliques et 10 fichiers de cri refaits, 0,76 $ (dont un passage perdu par un reset d'une autre session, refait depuis le cache) (section VX de `docs/budget.md`).
- Gascon (retiré des cris), quelques répliques galloises et flamandes, « À eux ! À eux ! » et « ¡Santiago! » gardent la
  lecture VO1 : ElevenLabs les rend mal (gascon inintelligible) ou whisper ne peut les juger.
- Outil : `voice_tts.py --shouts` (dépendances `faster-whisper`, `librosa` par `uv run --with`).
