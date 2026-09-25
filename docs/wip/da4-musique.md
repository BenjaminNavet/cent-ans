# DA4 — musique d'époque et bataille en couches

Lot DA4 (agent solo). Bible : `docs/design/2026-09-25-bible-da.md` § 9. ADR : `docs/decisions/0060-musique-d-epoque.md`.

## État : terminé

- [x] Recherche de sources libres (Wikimedia Commons, licence relue page par page via l'API
      `extmetadata.LicenseShortName` ; Freesound CC0 relu page par page comme AU1) : 11 nouvelles
      pistes (Machaut, Solage, Landini, Agincourt Carol, Sumer is Icumen In, Binchois, Dufay Ave
      Regina, Cantigas de Santa María) + 1 démonstration de chalemie (Schalmei) + 3 couches
      instrumentales de bataille (tambour, trompette droite, bourdon de cornemuse).
- [x] Téléchargement + conversion (MP3 128 kbit/s pour la musique, OGG Vorbis pour les couches ;
      normalisation `loudnorm` ~-16 LUFS) : `tools/cent_ans_tools/era_music.py`, idempotent,
      cache `~/.cache/cent_ans/era_music`.
- [x] `data/audio/music.json` (+ schéma `music.schema.json` révisé) : playlists par culture
      (`campaign_france/england/burgundy/iberia/italy`), `court`, `menu`, `war` ; structure
      `{primary, fallback}` (Kevin MacLeod rétrogradé en `fallback`, jamais supprimé) ;
      `culture_regions` (culture de faction → région musicale).
- [x] `data/audio/battle_layers.json` + schéma `battle_layers.schema.json` : couches (`melee_din`,
      `drums`, `straight_trumpet`, `bagpipe_drone`, `shawm`) et volumes par état d'intensité
      (approach/engagement/critical/victory/defeat), fade et hystérésis paramétrés.
- [x] `AudioDirector` (`game/scripts/audio/audio_director.gd`) : chargement `primary`/`fallback`,
      repli automatique, `culture_region()` / `campaign_context()` (lit `data/factions/<id>.json`),
      contexte `menu` à l'écran-titre.
- [x] `BattleMusicDirector` (`game/scripts/battle/battle_music.gd`) : lit désormais
      `battle_layers.json` au lieu de constantes codées en dur (couches, volumes, fade,
      hystérésis) ; API et hystérésis inchangées (tests B3 de `smoke.gd` toujours valides).
- [x] Test `tools/tests/test_era_music.py` (10 cas) : schémas, existence des fichiers référencés,
      crédit `SOURCE.md`, 5 régions ≥ 2 pistes, Kevin MacLeod jamais en `primary`, couches de
      bataille déclarées sur les 5 états.
- [x] `CREDITS.md` mis à jour (Kevin MacLeod retitré « repli » ; nouvelles pistes et couches).
- [x] `cargo` : aucun changement Rust. `uv run --project tools pytest` (suite complète) : vert.
      `uv run --project tools ruff check/format` : vert sur les fichiers touchés.
      `godot --headless --path game --script res://tests/smoke.gd` : exit 0.

## Limites connues

- Beaucoup de pièces d'Ars nova (Machaut, Landini, Binchois) ne sont disponibles sur Commons que
  sous forme de réalisations MIDI publiées comme domaine public (aucun enregistrement d'ensemble
  libre trouvé) : conservées et signalées comme telles dans `SOURCE.md`/`CREDITS.md`.
- Mélange des couches de bataille = volumes par palier d'intensité, pas une partition écrite pour
  la superposition (clause de repli explicitement prévue par le mandat DA4).
- Pas d'écoute humaine des niveaux (session sans sortie audio), comme déjà noté par AU1.
- Taille ajoutée : ~18 Mo (musique) + ~360 Ko (couches de bataille), très sous le plafond de 80 Mo.

## Revue de l'orchestrateur DA (26/09)

Les réalisations MIDI (Machaut ×2, Solage, Landini ×2, Binchois ×2) contredisent la bible § 9
(« pas de synthé ») : elles passent en `fallback`, avant Kevin MacLeod, et ne jouent donc que
si aucun enregistrement réel n'existe. France : `campaign.ogg`, estampie de Robertsbridge,
Dufay *Se la face ay pale* ; Bourgogne : Dufay ×2 ; Italie : *Chominciamento*, Ortiz.
Test `test_midi_renders_are_fallback_only`. Dette : trouver de vrais enregistrements libres
d'Ars nova (Machaut, Landini) pour la France et l'Italie.
