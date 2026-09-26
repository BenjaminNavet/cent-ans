# DA7a — vrais enregistrements libres d'Ars nova

Branche : `worktree-agent-a0b3b1e6efa501ff2`. Lot données seulement (pas de build Rust).

## Objectif
Remplacer en `primary` les rendus MIDI d'Ars nova (France, Italie) par de vrais enregistrements
interprétés sous licence libre (PD, CC0, CC BY, CC BY-SA ; jamais NC/ND). MIDI conservé en repli.

## État : terminé (en attente de merge)
- [x] Squelette + pipeline `tools/cent_ans_tools/ars_nova.py` (licence relue via l'API Commons,
      découpe, OGG Vorbis q5, loudnorm -16 LUFS, `SOURCE.md` et `.import` Godot générés)
- [x] Recherche : Commons (catégories + recherche plein texte), archive.org (API, filtre licence),
      FMA, Musopen — détail et rejets dans l'ADR 0060 § DA7a
- [x] 10 pistes dans `game/assets/third_party/music/ars_nova/` (bande Studio der frühen Musik
      1963, domaine public ; « Bel fiore dança », CC BY 4.0)
- [x] `data/audio/music.json` (France 4, Italie 3, Bourgogne 2, Angleterre 1, cour 3, menu 1,
      campagne 2), `CREDITS.md`
- [x] Test `tools/tests/test_ars_nova_recordings.py`, ADR 0060 § DA7a

## Points ouverts
- Écoute humaine : vérifier l'attribution titre ↔ plage de la bande de 1963 (ordre du programme)
  et les niveaux.
- Bande d'orgue Gotthard Arnér 1966 (Commons, domaine public, contient Landini « Angelica
  biltà » et le *Lamento di Tristano*) : à découper après écoute (plages ≠ programme).
- Machaut, Landini, Solage : toujours sans enregistrement réel libre (MIDI en repli).

## Reprise
`uv run --project tools python -m cent_ans_tools.ars_nova` (idempotent ; cache
`~/.cache/cent_ans/era_music/ars_nova`).
