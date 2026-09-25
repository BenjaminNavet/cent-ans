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
- [ ] **Génération bloquée** : `OPENAI_API_KEY` refusée par l'API (HTTP 401 « Incorrect API key »)
      dès le premier clip. Estimation du lot complet : 294 clips, ≈ 0,37 $. Rien facturé.
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
- [x] Test headless `game/tests/vo1_voice_test.gd` OK ; smoke 24 « smoke OK », au1/ub1/bv3 OK, capture du discours fenêtrée OK, pytest 382 OK.

## Prochaine étape
Faire passer `vo1_voice_test.gd` et le smoke. Quand une clé OpenAI valide est disponible :
`uv run --project tools python -m cent_ans_tools.voice_tts --dry-run` puis sans `--dry-run`
(≈ 0,37 $), `godot --headless --path game --import`, consigner le coût réel (somme `cost_usd` du
manifeste) dans `docs/budget.md`, commiter `game/assets/audio/voice/`.
