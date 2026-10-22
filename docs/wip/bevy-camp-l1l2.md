# bevy-camp-l1l2 : fondus de vie (L1) et statistiques de dump (L2)

Etat : code écrit, vérifications en cours.

- L1 : `life/models.rs apply_fades` et `life/stream.rs update_targets` lisent avant d'écrire ; un fondu réglé ne marque plus `Fade`/`Transform` changés. Tests `fade_tests` (models.rs).
- L2 : nouveau schedule `DumpPrepare` (`agent/dump.rs`), exécuté par `prepare_dump(world)` juste avant `build_dump` (script `dump`, BRP `cent_ans/dump`). Y tournent : `markers::update_stats`, `markers::probe_nearby`, `markers lod::batch_stats`, `life::update_stats`, `flipbook::count_sprites`. Plus rien en `Update`. `flipbook::count_live` (léger) reste par image car `FlipbookLooks::live()` sert à `battle/effects/dust.rs`.
- Limite : la commande console `dump-state` (`devtools.rs`, `&World`) ne rafraîchit pas ces stats.

Prochaine étape : fmt, clippy (x2), tests, dump en run_bg.
