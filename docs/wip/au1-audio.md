# AU1 — audio (banque libre T4, bataille spatialisée T3, ambiances de carte, mixage)

Agent AU1, vague 5 (session de nuit 7). Sources : `docs/audit/a5-technique.md` (§ 3, lots T3/T4),
`docs/audit/a4-assets-libres.md` (§ 6), `docs/wip/b3-musique-camera.md`, `docs/wip/d0-assets.md`.

## État

- [x] Squelette : bus (`AudioBuses`), banque (`SoundBank` + `data/audio/sound_bank.json` + schéma),
      `BattleAudio` (pool 3D, API statique), `CampaignAmbience`, outil `tools/cent_ans_tools/audio_bank.py`.
- [x] Lot 1 — banque sonore libre : 70 fichiers (`game/assets/audio/battle/`, `ambience/`, 7,6 Mo)
      tirés de 65 sons Freesound **CC0 vérifiés page par page** (aperçu HQ public, même licence),
      découpés / bouclés / mélangés par `audio_bank.py` (reproductible). Crédits : `CREDITS.md`,
      détail par fichier : `game/assets/audio/SOURCE.md`. Tests : `tools/tests/test_audio_bank.py`.
- [x] Lot 2 — audio de bataille spatialisé : `game/scripts/audio/battle_audio.gd`, branché dans
      `battle_scene.gd` (`_update_audio`) ; la couche 2D `sword_clash` de `battle_music.gd` (B3)
      est coupée quand `BattleAudio` est actif.
- [x] Lot 3 — ambiances de carte : `game/scripts/audio/campaign_ambience.gd`, créé par
      `AudioDirector.attach_campaign`.
- [x] Lot 4 — mixage : curseurs par bus dans Réglages (`settings_menu.gd`) et dans « Son… » de la
      carte ; ducking de la musique (`AudioDirector.duck_music`, déclenché par la banque).
- [x] Test headless `game/tests/au1_audio_test.gd` (bus, banque, pool, événements, siège,
      ambiances sur la vraie carte, volumes) : OK.
- [x] Smoke complet (exit 0, aucune erreur de script) ; passes fenêtrées `--audio-driver Dummy`
      (bataille `--autoplay --benchmark --weather=snow`, siège `--siege --bench-at=60
      --weather=rain`) sans erreur de script, 58-60 i/s médian.

## État : terminé

Prochaine étape éventuelle : réglage des niveaux à l'oreille dans une session avec écoute ;
BV1 peut se brancher sur `BattleAudio.play_at` (ci-dessous).

## Architecture

- **Bus** (`AudioBuses.ensure_layout()`, idempotent, appelé par `AudioDirector._ready`) :
  Master (limiteur dur −0,5 dB) ← Musique (amplification de ducking + compresseur côté-chaîne sur
  Voix) ← BatailleMusique (B3) ; Ambiance (compresseur doux) ; Bataille (compresseur) ←
  BatailleLointain (passe-bas + réverbération selon la hauteur de caméra) ; Interface ; Voix.
  L'ancien bus « Effets » est remplacé par « Interface » (clics, pages, cloche de tour).
- **Volumes** : `AudioDirector.set_bus_volume(bus, 0..1)` / `bus_volume(bus)`, persistés dans
  `user://settings.cfg` (`[audio] bus_<nom>` ; `music_volume` / `sfx_volume` conservés).
- **Banque** : `data/audio/sound_bank.json` (schéma `data/schemas/sound_bank.schema.json`) :
  `events` (variantes, bus, priorité 0-10, volume, hauteur aléatoire, instances max, recharge,
  `unit_size_m`, `max_distance_m`, `layers` pour les composites, `duck_music_db`/`duck_seconds`),
  `beds` (nappes 3D en boucle), `ambience` (boucles 2D).
- **Pool** : 28 `AudioStreamPlayer3D` ; pool plein → vol de la voix la moins prioritaire puis la
  plus ancienne si sa priorité ≤ celle du nouveau son ; limite d'instances → remplace la plus
  ancienne du même événement. Au-delà de 70 m, bus « BatailleLointain ».
- **Nappes** : mêlée (2 couches), clameur, marche, galop, feu ; volume = √(soldiers pondérés par
  la distance au point visé / seuil), barycentre glissant ; vue haute → nappe 2D
  `battle_distant` étouffée.
- **Événements déduits** (`BattleAudio._detect_events`) : passage en charge (cri, hennissement,
  cri de guerre au premier engagement d'un camp), contact (boucliers + épées), baisse de munitions
  (décoche / arbalète / trébuchet / bombarde, sifflement à mi-course, impact à l'arrivée calé
  sur la vitesse des traits), pertes en mêlée (râles, tirage aléatoire), déroute, mort d'un
  général (cor + clameur, musique atténuée) ; siège (`update_siege`, 4 Hz) : bélier (PV de la
  porte), impacts (PV des murs), effondrement, cloche d'alarme les 150 premières secondes,
  nappe de feu sur la maison en flammes la plus proche ; météo : pluie, vent, vent fort (neige),
  tonnerre sous la pluie.
- **Campagne** : grille 3 × 3 autour du point visé (rayon ∝ zoom) → mer (`land_mask`), forêt /
  champs (`splat`), ville (capitales + `settlements_px.json`) ; `layer_levels` (pure) croise
  zoom, saison (date du tour) et météo (tirage saisonnier déterministe par date, la carte n'ayant
  pas de météo simulée ; `weather_override` pour l'imposer).

## API pour BV1 (volées de flèches visuelles)

```gdscript
# Joue un événement de la banque à une position monde (bataille en cours) ; false si aucun
# BattleAudio actif, événement inconnu, trop loin, en recharge ou pool saturé.
BattleAudio.play_at("arrow_whistle", position)            # sifflement de volée
BattleAudio.play_at("arrow_impact", impact_pos, -3.0)     # gain optionnel en dB
BattleAudio.play_at_delayed("arrow_impact", impact_pos, flight_time_s)  # temps de bataille
```

Événements : `sword_clash`, `shield_bash`, `contact`, `arrow_impact`, `arrow_whistle`,
`bow_release`, `crossbow_release`, `charge_cry`, `war_cry`, `death_groan`, `rout_cry`,
`horse_neigh`, `horn`, `drum`, `bell_toll`, `ram_hit`, `trebuchet_release`, `stone_impact`,
`bombard`, `wall_collapse`, `thunder`, `general_death` (voir `data/audio/sound_bank.json`).
`BattleAudio` produit déjà lui-même les sons de volée à partir des munitions des régiments ; si
BV1 joue ses propres sifflements/impacts, poser `BattleAudio.auto_volley = false` (propriété
statique) pour éviter le doublon. Le pool, les bus et la banque appartiennent à AU1.

## Régénérer la banque

`uv run --project tools --with soundfile python -m cent_ans_tools.audio_bank [clip…]`
(cache des téléchargements : `~/.cache/cent_ans/freesound`). Chercher des candidats CC0 :
`python3 tools/cent_ans_tools/freesound_search.py "horse neigh"`. `soundfile` (libsndfile) est
tiré à la volée (`--with`) : pas de dépendance ajoutée au projet `tools/`.

## Limites connues

- Niveaux et choix des extraits réglés sans écoute (session headless) : à affiner à l'oreille
  (volumes de `sound_bank.json`, fenêtres de `audio_bank.py`). Enregistrements Freesound
  modernes : quelques ambiances (campagne, ville) peuvent contenir des bruits anachroniques
  lointains.
- Aperçus Freesound à 128 kbit/s (l'original exige un compte connecté) : qualité suffisante pour
  des sons de jeu, pas pour des gros plans.
- Pas de météo simulée sur la carte de campagne : tirage saisonnier purement sonore.
- Pilote audio factice (`--audio-driver Dummy`) : à la sortie, Godot signale des flux OGG
  encore référencés (dont `music/war.ogg` de B3, préexistant) : lectures non libérées par le
  pilote factice, sans effet en jeu.
- Les événements de bataille n'ont pas de « kind » côté Rust : tout est déduit des transitions
  d'état des régiments (pas de cri de guerre sur ordre du joueur).
