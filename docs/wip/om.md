# OM — carte Oural–Méditerranée (orchestration)

Spec `docs/superpowers/specs/2026-09-28-oural-mediterranee-design.md`, plan
`docs/superpowers/plans/2026-09-28-oural-mediterranee.md`, ADR 0115-0116.
Worktree d'intégration `../game_project-om` (`feat/om`). Lancé le 2026-09-28 à 23 h, joueur absent
jusqu'au matin (autonomie complète).

## État
- OM0 fait (squelette).
- Vague 1 lancée : OM1, OM2, OM3, D1, D2, D3.
- D3 fini (feat/om-d3, 99b7e127) : 43 provinces, 18 factions ; Smolensk sous la Horde à relire (orbite lituanienne ?). D4 lancé (../gp-om-d4).
- D1 (33 prov, 12 fac), D2 (51 prov, 15 fac), D3 fusionnés dans feat/om (5ca326bb, 9375189f) : résolveur JSON à trois voies (scratchpad json3.py, copié dans tools/ à la fin), relations croisées, Kuyavie teutonique. 313 provinces, 136 factions. D5, D6 lancés.

## Reprise
Lire ce fichier, `git log feat/om`, les notes `docs/wip/om-*.md` des lots.
- OM1, OM3 fusionnés ; D5 fusionné (48 prov, 23 fac ; 4869e690). P1 (blasons/portraits D1-D3, 5 $) lancé.
- À faire à la fusion D4 : relations Byzance–Ottomans (guerre), Byzance–Aydın/Saruhan, Hospitaliers–Aydın/Menteşe (guerre), Chypre–Mamelouks, Trébizonde (cul_greek défini par D4) ; D6 : Mérinides–Castille (guerre), Hafsides–Aragon/Sicile.
- D6 fusionné (37 prov, 6 fac ; 93cc38fb). Total 443 provinces, 177 factions. C1 (données annexes) lancé. Reste : OM2 géo, P1, puis P2 portraits D4-D6, intégration complète.
- P1 fusionné (écus/bannières toutes factions, 46 portraits + 37 âgés D1-D3, 3,85 $). P2 (portraits D4-D6, ≤ 4,50 $) lancé. Attente OM2.
- OM2 fusionné (artefacts géo 7168×6144, 406 prov). I1 (intégration : régénération 443 prov, corrections, tous tests, mesures) lancé dans ../gp-om-om2. Ensuite : fusion P2, puis main.
- P2 fusionné (49 portraits + 42 âgés D4-D6, 4,20 $ ; cumul OM 8,05 $/10). Contrôle visuel planche : correct ; Andronic III en couronne occidentale, Abu l-Hasan auréolé (à retoucher éventuellement).
- I1 fini sur feat/om-om2 (tests Rust 1223 ok, pytest 1246 ok, 22 tests Godot carte ok) ; fusionné dans feat/om, puis main (FE8, Q6, DZ) fusionné dans feat/om sans conflit textuel. Vérification complète post-main relancée par I1 (reprise). Mesures : chargement 6,7 → 11,4 s, RSS 1,61 → 2,89 Go, planification IA ≈ 0,23 → 1,07 s par tour (après correctif are_neighbors).
- Vérification post-main verte (Rust 1223, pytest 1246, smoke 30/30, 22 tests carte + tests Q6/DZ). century_probe 50 tours × 5 graines : banqueroutes 5,25/fac./déc. (petites factions surtout ; anciennes 0,03-0,23), révoltes 15,2/200 tours, commise de Guyenne au tour 1 sur 5/5 graines.
- **Incident** : `q3_playtest.gd` lancé en headless par I1 a réécrit `settings.cfg` du joueur (fenêtré 1920×1080, conseiller et voix réactivés, `advisor_seen` vidé) ; anciennes valeurs perdues.

## Reste (après fusion dans main)
- Sonde 464 tours × 10 graines et équilibre des petites factions de l'Est (banqueroutes, révoltes).
- Relecture historienne : 31 objectifs ajoutés par I1, incertitudes listées dans om-d1..d6.
- Coût IA ×4,6 par tour (recherche de chemins, faction_power) ; chargement 11,4 s ; RSS 2,9 Go (relief_shade à tuiler/compresser).
- Relief fin (tiers 1-3) absent à l'Est ; unités orientales ; musique orthodoxe/orientale ; portraits : Andronic III en couronne occidentale.
