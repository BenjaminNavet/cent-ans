# VO1 — voix (répliques d'unités, discours du général, conseiller)

Agent VO1 (session 7). Branche `worktree-agent-ae1c20a86b355d930`.
Sources : `docs/wip/au1-audio.md` (bus Voix, `BattleAudio.play_at`, `duck_music`),
`docs/wip/bv3-finitions-bataille.md` (discours), `docs/wip/ub1-interface-bataille.md`.
Plafond de dépense du lot : **3 $** (estimation avant tout appel, consignée dans `docs/budget.md`).

## État
- [x] Corpus : `data/voice/barks.json` (répliques), `data/voice/advisor.json` (conseiller),
      `data/voice/speech_voices.json` (voix des discours) + schémas `data/schemas/voice_*.schema.json`,
      test `tools/tests/test_voice.py`.
- [x] Outil `tools/cent_ans_tools/voice_tts.py` (cache, `--dry-run`, plafond, manifeste).
- [ ] Génération (sonde puis lot complet), consignation budget.
- [ ] Lecteurs Godot : `BattleVoices` (répliques), voix du discours (`battle_speech.gd`), `Advisor`.
- [ ] Réglage « Conseiller : activé / désactivé », crédits, test headless, smoke.

## Prochaine étape
Sonde de génération (5 clips), écoute des niveaux par ffprobe, puis lot complet.
