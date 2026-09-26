# DA7a — vrais enregistrements libres d'Ars nova

Branche : `worktree-agent-a0b3b1e6efa501ff2`. Lot données seulement (pas de build Rust).

## Objectif
Remplacer en `primary` les rendus MIDI d'Ars nova (France, Italie) par de vrais enregistrements
interprétés sous licence libre (PD, CC0, CC BY, CC BY-SA ; jamais NC/ND). MIDI conservé en repli.

## État
- [ ] Squelette (module `tools/cent_ans_tools/ars_nova.py`, test, ce fichier)
- [ ] Recherche des sources (Commons, Musopen, archive.org, FMA, IMSLP)
- [ ] Téléchargement + conversion OGG Vorbis, loudnorm -16 LUFS
- [ ] `data/audio/music.json`, `SOURCE.md`, `CREDITS.md`
- [ ] Test pytest, ADR 0060 § DA7a

## Prochaine étape
Recherche des sources.
