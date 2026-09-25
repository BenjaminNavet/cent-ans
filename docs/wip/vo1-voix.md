# VO1 — voix (répliques d'unités, discours du général, conseiller)

Agent VO1 (session 7). Branche `worktree-agent-ae1c20a86b355d930`.
Sources : `docs/wip/au1-audio.md` (bus Voix, `BattleAudio.play_at`, `duck_music`),
`docs/wip/bv3-finitions-bataille.md` (discours), `docs/wip/ub1-interface-bataille.md`.
Plafond de dépense du lot : **3 $** (estimation avant tout appel, consignée dans `docs/budget.md`).

## État
- [x] Corpus : `data/voice/barks.json` (120 répliques : français 35, anglais 35, anglo-normand 12,
      gascon 10, flamand 10, gallois 8, écossais 10), `data/voice/advisor.json` (21 interventions
      de Jean le Bel), `data/voice/speech_voices.json` (voix des discours par faction) ; schémas
      `data/schemas/voice_*.schema.json` ; test `tools/tests/test_voice.py`.
- [x] Outil `tools/cent_ans_tools/voice_tts.py` : tâches tirées des données, cache (fichier
      existant jamais régénéré, réponse brute gardée dans `~/.cache/cent_ans/tts`), `--dry-run`
      (estimation), plafond `--cap` (3 $), ffmpeg (silences coupés, réverbération de salle pour le
      conseiller, −16 LUFS, OGG mono), manifeste `game/assets/audio/voice/manifest.json`
      (voix, durée, coût mesuré).
- [x] **Génération (reprise)** : OpenAI refusée (401) → backend OpenRouter `openai/gpt-audio-mini`
      (chat completions, `modalities: [text, audio]`, flux SSE obligatoire, pcm16 24 kHz reconstitué).
      Consigne « lire mot pour mot » + exemple, contrôle de chaque clip (durée plausible, transcription
      ≈ texte, sans didascalie), reprises (`--attempts`, `--recheck`). 294 clips (20,6 min), coût
      réel 0,24 $ (somme `usage.cost`, rejets compris), consigné en session 7.
- [x] Lecteurs Godot (fonctionnent sans fichiers : sous-titres seuls, silence) :
  - `game/scripts/audio/voice_lines.gd` (`VoiceLines`) : données, langue d'un régiment, repli,
    voix d'un général, nom des fichiers de discours (sha1 du texte).
  - `game/scripts/audio/battle_voices.gd` (`BattleVoices`) : sélection, marche, attaque (2D, bus
    Voix), charge, déroute, chute du général (3D via le pool de `BattleAudio`, événements `vo_<id>`
    déclarés à la volée, bus Voix), victoire ; une réplique à la fois, priorités, écart global,
    recharge et probabilité par situation ; muet pendant le discours et le conseiller.
  - `battle_speech.gd` : voix du général par phrase et pour le cri, durée des phrases calée sur la
    voix (sous-titres synchronisés), musique atténuée.
  - `game/scripts/audio/advisor.gd` (`Advisor`) : premier tour (par faction), première bataille /
    premier assaut (après le discours), premier siège, première victoire / défaite, alertes du tour
    pour la faction du joueur (une par tour, `alert_cooldown_turns`) ; sous-titre en haut à gauche,
    clic pour faire taire ; file de 2.
- [x] Réglages (onglet Son) : « Conseiller » (`voice/advisor`), « Répliques des unités »
      (`voice/barks`) ; le curseur « Voix » existait (AU1).
- [x] Crédits (`CREDITS.md`), budget (ligne VO1, 0,00 $ réel).
- [x] Test headless `game/tests/vo1_voice_test.gd` OK ; smoke 24 « smoke OK », au1/ub1/bv3 OK, capture du discours fenêtrée OK, pytest 398 OK (après fusion de main).

## Prochaine étape
Terminé. Écoute humaine à faire : quelques lectures un peu théâtrales (allongements, pauses) ;
pour refaire un clip, supprimer son `.ogg` et son entrée du manifeste (ou `forget()`), puis
`uv run --project tools --with soundfile python -m cent_ans_tools.voice_tts --cap 3`.

## Points ouverts
- Voix OpenRouter : le modèle, conversationnel, improvise parfois (rejets automatiques) ; accents
  gascon, gallois, flamand approximatifs ; « plaçaes » dans `adv_start_france` (transcription).
- Le ducking du discours dure le discours entier même s'il est passé (`duck_music` ne s'annule
  pas).
- Les phrases du général contenant son nom (`general_lines`) ne sont pas vocalisées.
- Factions sans langue propre (Castille, Aragon…) parlent français ; la Bretagne aussi.
- Niveaux des voix à régler à l'oreille après génération.
- `ruff check tools` signale 6 erreurs dans `tools/blender_scripts/` (autres lots, pas VO1).
