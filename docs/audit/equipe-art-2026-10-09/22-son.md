# Sound designer — état des lieux (09/10, lecture seule, rien écouté)

## 1. État actuel
**`game/assets/audio/`** : 435 fichiers (425 OGG, 10 WAV).
- `battle/` (105, 4,3 Mo) : 31 familles (épées, efforts, râles, boucliers, armures, chutes, flèches, arbalète, chevaux, siège) + 7 boucles de fond.
- `ambience/` : 9 boucles (mer, campagne, grillons, forêt, ville, vent ×2, pluie, bataille lointaine).
- `ui/` (10 WAV découpés), `sfx/` (10 procéduraux).
- `voice/` (12 Mo, 294 clips, 0,16 $) : 120 répliques en 7 langues, 21 phrases du conseiller, 153 phrases de discours ; cris ElevenLabs v3 (ADR 0145), reste gpt-audio-mini.
- Musique : 3 pistes synthétiques de repli, 4 couches de bataille de 8 s, 172 Mo tiers (Wikimedia 98, Kevin MacLeod 58, Ars Nova 17).

**Scripts** `game/scripts/audio/` (2 655 l.) : `audio_director`, `audio_buses`, `battle_audio` (pool 3D), `battle_voices`, `campaign_ambience`, `ui_sounds`, `advisor`, `voice_lines`, `voice_pool`, `sound_bank`.
**Données** : `music.json` (12 contextes, 55 cultures → régions), `battle_layers.json`, `sound_bank.json` (38 événements).
**Mixage** : bus créés en code ; limiteur Master ; ducking musique sous la voix ; compresseurs ; BatailleLointain filtré selon le zoom ; 6 curseurs.
**ADR** : 0060, 0145, 0154, 0166, 0235.

## 2. Forces
- Licences traçables (`SOURCE.md`), jamais NC/ND ; `CREDITS.md` à jour.
- Pipelines reproductibles avec cache et plafond.
- Ambiance de carte selon terrain, zoom, saison, météo.
- Bataille spatialisée ; musique pilotée (rotation persistante, culture, intensité avec hystérésis).
- Voix multilingues bon marché, contrôle auto (whisper, hauteur).

## 3. Faiblesses
- Sources dégradées : banque AU1 = aperçus Freesound MP3 128 kbit/s ; musique tierce idem.
- Couches de bataille de 8 s → boucle audible ; pas de stinger victoire/défaite.
- Musique de bataille : 2 pistes `primary` ; guerre : 3 ; 7 rendus MIDI (contre la bible « pas de synthé »).
- Aucun son naval.
- Bataille : une arbalète, pas de pique, une charge, galop seul en boucle, pas d'acclamations, pas d'ambiance avant contact, pas de canon à main.
- Carte : un seul son de ville ; pas de nuit, rivière, montagne, neige ; pas d'armée en marche ni de ville assiégée.
- UI : pas de survol ; rien pour guerre, paix, mort, mariage, excommunication.
- Voix : gascon, gallois, flamand mal rendus ; ducking du discours non annulé au saut ; IA non déclarée dans `CREDITS.md`.
- 58 Mo de Kevin MacLeod en repli seulement.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| 1 | Couches de bataille 30-60 s en variantes + stingers victoire/défaite/déroute | Fort | M | 0 | gameplay |
| 2 | 6 pistes `primary` bataille et guerre, sans MIDI | Fort | M | 0 | DA, crédits |
| 3 | Sons navals (si la flotte est jouable) | Fort | M | 0 | naval |
| 4 | Stingers diplomatiques et dynastiques | Moyen-fort | S | 0 | UI, `event_sfx` |
| 5 | Originaux WAV Freesound, regénérer AU1 | Moyen | S | 0 | outils |
| 6 | Bataille : arbalète, pique, canon à main, sabots par terrain, acclamations, ambiance avant contact | Moyen | M | 0 | animation, sim |
| 7 | Carte : nuit, rivière, montagne, neige, ville en 3 tailles, armée en marche | Moyen | M | 0 | carte |
| 8 | Ducking annulé au saut du discours | Moyen | S | 0 | UI bataille |
| 9 | Son de survol discret, sons de panneau | Faible-moyen | S | 0 | UI |
| 10 | Cris gascons/gallois/flamands refaits ; IA déclarée | Moyen | S-M | ~5-50 € | production |
| 11 | Musique OGG q5, Kevin MacLeod hors build | Faible | S | 0 | build |
| 12 | Composition originale en couches | Très fort | L | 2-10 k€ | production |

Une passe d'écoute humaine reste à faire sur tous les lots.
