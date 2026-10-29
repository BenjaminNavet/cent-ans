# Lot DATA (boucle juge, itération 1)

Source : `out/juge2/iter-01-tech.md` (sections données et audio).

## État
- [x] 1. `valley_courses.json` : le schéma était en retard sur le code (`valley_rivers.py` lit `scalerank`, `width_px`, `clip_land`, `to_sea` ; `colormap.py` lit `irrigated_areas`). Schéma complété, donnée inchangée.
- [x] 2. ids `names_*` : pattern `^names_[a-z]{2,3}$` (codes ISO 639-3) ; `names_nordic` renommé `names_non` (vieux norrois, ISO 639-3 `non`), fichier compris. Aucune autre référence à ces ids.
- [x] 3. Couverture : `SCHEMA_TABLE` étendue aux schémas manquants ; `test_every_schema_is_validated_or_exempt` empêche le retour. Helper `assert_matches_schema` lit aussi les YAML. Exceptions : `common` (défs), `battle_replay` (échantillon testé ailleurs).
- [x] 4. `war_horn` : entrée `events.war_horn` dans `data/audio/sound_bank.json`, `volume_db: -7` (mesuré : -9,6 → -16,6 LUFS). Priorité, instances et pitch fixés à l'ancien comportement brut.

## Points ouverts
- Choix de conception : pattern des ids élargi à 2-3 lettres (pour `ady`) plutôt que renommage ; à confirmer.
- Le test `test_every_bank_file_exists_and_is_generated` accepte désormais les effets `audio.SFX` (`sfx/<nom>`).
- Le juge a noté aussi `fanfare` à -12,7 LUFS : non traité (hors demande).
