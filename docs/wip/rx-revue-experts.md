# RX — revue par experts puis corrections

Demande joueur (10-09) : orchestrateur Opus, critiques Sonnet par rôle, critique constructive, puis corrections sans lui demander.
Mandat : méthode code + tests + captures (≈ 10 par critique visuel) ; corrections « tout, avec ADR », refontes lourdes en propositions ; enveloppe payante 5 $ (docs/budget.md). Démarré après clôture SC (01122276e).

## État
- [x] Vague critique (10 agents ; ia et perf encore en cours au lancement de la vague 1) : animation, assets3d, ui, campagne, bataille, mecaniques, ia, historien, audio, perf → `docs/wip/rx/<rôle>.md`
- [~] TX fini (eea65a370, import fait) ; critiques campagne-v2 et bataille-v2 lancés.
- [~] Vague 1 de corrections lancée (victory, probe, audio, histoire, uifin, anim) : worktrees `../gp-rx-<lot>`, branches `rx/<lot>`, table `docs/wip/rx/lots.md`, brief `docs/wip/rx/brief-lot.md`.
- [ ] (ancien) Après fin de TX (textures, session c5) : re-juger carte de campagne et bataille (nouveaux critiques campagne + bataille, + assets3d visuel), avant les corrections visuelles
- [ ] Synthèse → `docs/wip/rx/synthese.md` (lots)
- [ ] Vagues de corrections (≤ 10 agents, worktrees, ff-only)
- [ ] Vérif finale (fmt/clippy/test/pytest/smoke) + push

## Notes
- Disque 21 Go libres au départ : worktrees de correction avec `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`, `rm -rf core/target` en fin de lot.
