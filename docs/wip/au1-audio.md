# AU1 — audio (banque libre T4, bataille spatialisée T3, ambiances de carte, mixage)

Agent AU1, vague 5 (session de nuit 7). Sources : `docs/audit/a5-technique.md` (§ 3, lots T3/T4),
`docs/audit/a4-assets-libres.md` (§ 6), `docs/wip/b3-musique-camera.md`, `docs/wip/d0-assets.md`.

## État

- [x] Squelette : bus (`AudioBuses`), banque (`SoundBank` + `data/audio/sound_bank.json` + schéma),
      `BattleAudio` (pool 3D, API statique), `CampaignAmbience`, outil `tools/cent_ans_tools/audio_bank.py`.
- [ ] Lot 1 — banque sonore libre (Freesound CC0 vérifié page par page + mélanges procéduraux).
- [ ] Lot 2 — audio de bataille spatialisé (pool, nappes de mêlée, événements).
- [ ] Lot 3 — ambiances de carte de campagne.
- [ ] Lot 4 — mixage : curseurs par bus, ducking.

## Prochaine étape

Écrire le manifeste des sources et le pipeline de `audio_bank.py`, générer la banque.

## API pour BV1 (volées de flèches visuelles)

```gdscript
# Joue un événement de la banque à une position monde (bataille en cours) ; false si aucun
# BattleAudio actif, événement inconnu, trop loin, en recharge ou pool saturé.
BattleAudio.play_at("arrow_whistle", position)            # sifflement de volée
BattleAudio.play_at("arrow_impact", impact_pos, -3.0)     # gain optionnel en dB
BattleAudio.play_at_delayed("arrow_impact", impact_pos, flight_time_s)
```

Événements disponibles : voir `data/audio/sound_bank.json` (`events`). `BattleAudio` produit déjà
lui-même les sons de volée à partir des munitions des régiments ; si BV1 joue ses propres
sifflements, poser `BattleAudio.auto_volley = false` (propriété statique) pour éviter le doublon.
