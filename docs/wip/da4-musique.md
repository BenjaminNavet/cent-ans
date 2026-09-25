# DA4 — musique d'époque et bataille en couches

Lot DA4 (agent solo). Bible : `docs/design/2026-09-25-bible-da.md` § 9. Existant : `AudioDirector`
(`game/scripts/audio/audio_director.gd`), `BattleMusicDirector`
(`game/scripts/battle/battle_music.gd`), `data/audio/music.json`, tiers `kevin_macleod/` et
`wikimedia/` déjà en place avec `SOURCE.md`.

## État

- [x] Recherche de sources libres (Wikimedia Commons, vérification licence page par page via
      l'API `extmetadata.LicenseShortName`) : Ars nova France (Machaut), Italie (Landini,
      Chominciamento di gioia), Angleterre (Agincourt Carol, Sumer is Icumen In), Bourgogne
      (Binchois, Dufay), Ibérie (Cantigas de Santa María, Folía d'Ahigal, Ortiz). Freesound CC0
      pour les couches de bataille (tambour, cornemuse/sackpfeife, chalemie, reconstitutions
      historiques Jorvik/reenactment).
- [ ] Téléchargement + conversion (ogg, normalisation ~-16 LUFS) : `tools/cent_ans_tools/era_music.py`.
- [ ] `data/audio/music.json` : playlists par culture (france/england/burgundy/iberia/italy) +
      cour + menu, structure `{primary, fallback}` (Kevin MacLeod en repli).
- [ ] `data/audio/battle_layers.json` + schéma : couches (base, tambour, trompette droite,
      bourdon de cornemuse, chalemie) par état d'intensité.
- [ ] `AudioDirector` : sélection de la liste de lecture par culture du joueur, repli primary→
      fallback.
- [ ] `BattleMusicDirector` : lecture des couches depuis `battle_layers.json` au lieu des
      constantes codées en dur.
- [ ] Test (pytest ou GDScript) : chaque piste référencée existe + a un `SOURCE.md`.
- [ ] ADR `docs/decisions/00NN-musique-d-epoque.md`.
- [ ] `CREDITS.md` mis à jour, Kevin MacLeod retitré « repli ».

## Prochaine étape

Écrire `tools/cent_ans_tools/era_music.py`, télécharger, remplir `data/audio/`.
