# RX — revue par experts puis corrections

Demande joueur (10-09) : orchestrateur Opus, critiques Sonnet par rôle, critique constructive, puis corrections sans lui demander.
Mandat : méthode code + tests + captures (≈ 10 par critique visuel) ; corrections « tout, avec ADR », refontes lourdes en propositions ; enveloppe payante 5 $ (docs/budget.md). Démarré après clôture SC (01122276e).

## État — CLÔTURE (10-10)
- [x] Vague critique (12 rapports `docs/wip/rx/<rôle>.md`, dont campagne-v2 et bataille-v2 après TX).
- [x] Corrections : 14 lots tous fusionnés dans main (table `docs/wip/rx/lots.md`, ADR 0245-0259), worktrees et branches supprimés.
- [x] Correctifs d'intégration : smoke déploiement (refus dans le bandeau, 291bb5a10), Cambridge + test bâtisseur (79ad774e3).
- [x] Vérif finale (fmt/clippy/test/pytest/smoke) : voir § Vérification finale.
- Dépense payante : 0 $ (enveloppe 5 $ non utilisée).

## À juger par le joueur
- Audio : boucles et clic entre pistes (fondu 3 s, normalisation de gain), cris de bataille par langue.
- UI (uifin) : barre d'emplacements, écran de résultat, cartes de faction, panneau de colonie (masque encore la carte), chiffres alignés, apostrophes typographiques.
- Bataille : neige (plus de stries, mais sol gris sous la brume), contours de mêlée, contours d'aperçu au-dessus d'une rivière (non vus à l'œil), caméra de déploiement et gros plan.
- Campagne : palette Oural plus verte/dorée, nuages plus légers, lavis diplomatique, fleuves du parchemin.
- Difficulté : leviers IA (ADR 0258) réglés sans partie jouée par niveau.

## Restes (voir chaque ligne de `docs/wip/rx/lots.md`)
- Équilibrage : ~¼ des factions en guerre, durée des guerres = plancher, banqueroutes ; FR-EN graine 2 hors bande ; sièges pris 55 % (< 60 %).
- Visuel : toits ardoise étirés (proposition `fit_scale`), sol olive de Paris (parcelles, périmètre TX), Grand Perm = lavis allié coupé au bord.
- Robustesse : `Parameter "t" is null` (paquet DN), fuites en sortie, tests de mise en page UI (po_ui, p2b, p2g, da7d).
- Audio : voix pour 65 factions sans langue de cri, mp3→ogg.

## Notes
- Disque 21 Go libres au départ : worktrees de correction avec `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`, `rm -rf core/target` en fin de lot.

## Vérification finale (10-10)
- `cargo fmt --check`, `clippy --workspace --all-targets -D warnings`, `cargo test --workspace` : verts.
- `smoke.gd` complet : 32 étapes OK, 0 erreur de script ou de shader (après `--import` et reconstruction de la dylib : les classes `ExpectedErrors` et `MissionTracker` n'étaient pas dans le cache local).
- pytest : 3389 réussis ; échecs restants hors RX : `test_budget` (lignes de cumul saisies à la main par DN), `test_settlement_graph` ×2 et `test_settlements_schema[prov_bar]` (déjà en échec avant RX : relancer `geo roads` puis `geo hamlets`). `test_icons` corrigé par WH (af7d60fde).
