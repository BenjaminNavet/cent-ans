# VX — voix de combat criées

Retour joueur (30/09) : « la voix était un peu molle en combat ; Montjoie, Saint-Denis ».

## Diagnostic
gpt-audio-mini (VO1) lit les cris à hauteur de parole : « Montjoie ! Saint-Denis ! » à
~130 Hz médians, comme « Monseigneur ? » calme (114 Hz). Un cri d'homme monte à 200-350 Hz.
Sonde ElevenLabs v3 (fal.ai, balise `[shouting]`) : 250-360 Hz, transcription juste, accent
français comparable (whisper fr 0,8-0,96). Sur les répliques calmes, léger accent anglais
(« Monsignor ») → sélection et marche restent en OpenRouter.

## Décisions
- Situations criées (attaque, charge, déroute, victoire, chute du général) : ElevenLabs v3,
  balise d'émotion par situation (`shout_tag` dans `data/voice/barks.json`), voix
  `shout_voices` par langue.
- Cris de guerre (fin du discours) : chœur de 5 voix × 2 couches (décalage ≤ 120 ms,
  hauteur ±6 %, gain), écho de plein air (`cry_chorus` dans `speech_voices.json`).
- Contrôle local de chaque prise : whisper (mots ou lettres), langue ≥ 0,5, F0 médian ≥ 180 Hz,
  durée ; 3 prises au plus, sinon l'ancien clip reste.
- Traitement combat : passe-haut, compression, présence 2,6 kHz, −13 LUFS (−16 pour la parole).

## État
- [x] Données, schémas, `tools/cent_ans_tools/voice_shout.py`, branchement `voice_tts.py`.
- [ ] Génération : `uv run --project tools --with soundfile --with faster-whisper --with librosa python -m cent_ans_tools.voice_tts --only barks --only speech`
- [ ] Tests pytest (jobs criés, périmé si modèle différent), vo1_voice_test, smoke.
- [ ] Budget, crédits, écoute joueur.
