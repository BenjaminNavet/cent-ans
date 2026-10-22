# Lot A2 — audio : anachronismes et crédits CC BY

Source : `docs/wip/nuit-critiques/audio.md` (constats 1 et 2 de la nuit du 2026-10-22).

## État

- Fait : retrait des pistes anachroniques citées (Bach, Hume, Le Roy, Mudarra, maqams 1931) de
  `data/audio/music.json` ; fichiers conservés sur le disque ; entrées `track_gain_db` devenues
  orphelines supprimées.
- Fait : `data/audio/credits.json` (57 pistes tierces jouées, générées depuis les `SOURCE.md`),
  schéma `data/schemas/audio_credits.schema.json`, section « Musique » ajoutée à l'écran Crédits
  (`client/src/menu/music_credits.rs`, `main_menu.rs::spawn_credits`).
- Tests : `tools/tests/test_music_playlists.py` (crédit en jeu de chaque piste tierce jouée,
  17 MacLeod en CC BY 4.0), `tools/tests/test_schemas.py` (nouvelle ligne du tableau) ;
  tests Rust `menu::music_credits`.

## Points ouverts (choix non tranchés)

- Anachronismes non cités par le constat, à décider : Gibbons, Cabezón, Ortiz, Kapsberger,
  Moulinié (XVIe-XVIIe s.), Greensleeves (fin XVIe s.), Istikhbar Mezmoum (1929), Hüseyni saz
  semaisi (avant 1939).
- Listes amincies par le retrait, non complétées : `war` (1 piste primaire), `campaign_burgundy`
  (2 primaires : Moulinié, Tomsińska). Candidats déjà sur disque et créditables : Agincourt
  Carol (non référencé), Dufay « Ave Regina », Dufay « Se la face ay pale », Binchois/Dufay
  « Adieu m'amour » (ars nova, en repli aujourd'hui).
- `campaign_islamic` n'a plus que 2 pistes primaires (Istikhbar 1929, Hüseyni pré-1939).
- Znamenny (chant russe) conservé : non cité dans le constat, à trancher.
- Les fichiers retirés restent dans `game/assets/third_party/music/wikimedia/` et leurs
  `SOURCE.md` ne sont pas modifiés ; `CREDITS.md` (racine) les mentionne toujours.
- Crédits Wikimedia BY-SA : vérification « partage à l'identique » non faite (hors constat 1).
