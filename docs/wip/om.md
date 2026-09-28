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
