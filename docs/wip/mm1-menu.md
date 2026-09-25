# MM1 — première impression : écran titre, menu, choix de faction, chargement, introduction

Agent MM1 (session 7). Sources : `docs/audit/a3-ui.md` (§ 3.1, M1-M6), `docs/wip/au1-audio.md`,
`docs/wip/v3-atmosphere.md`, ADR 0014 (figurines skinnées) et 0015 (Paris).

## Plan
1. Données : `data/ui/front_end.json` + schéma `data/schemas/front_end.schema.json` + test
   `tools/tests/test_front_end_schema.py` ; lecteur `game/scripts/ui/front_end_data.gd`.
2. Décor 3D du menu : `game/scripts/ui/menu_backdrop_3d.gd` — Paris L1 (`paris_siege.glb`) au
   crépuscule (ciel HDRI V3 « dawn »), plans de caméra lents enchaînés en fondu, ost au premier plan
   (figurines V2 skinnées en `MultiMesh`, bannières animées).
3. Menu principal refait (`start_menu.tscn/.gd`) : titre enluminé, colonne de boutons (Nouvelle
   partie, Continuer, Charger, Codex, Réglages, Crédits, Quitter), fondus.
4. Choix de faction (`faction_select.gd`) : grandes cartes (blason, souverain, intro, forces et
   faiblesses, difficulté), date de départ, graine rangée dans « Options avancées ».
5. Écran de chargement : illustration encadrée, citation datée, conseil, barre réelle.
6. Introduction passable (`intro_cards.gd`) : 5 cartons 1328-1337.
7. Captures `docs/audit/captures/mm1/`, mesures FPS et temps de chargement.

## État
- [x] Étape 1 (données, schéma, test : 3 OK)
- [ ] Étape 2
- [ ] Étapes 3-7

## Prochaine étape
Décor 3D du menu.
