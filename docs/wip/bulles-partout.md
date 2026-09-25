# WIP — Bulles partout (session historien, 25/09)

Conception : `docs/design/2026-09-25-bulles-partout.md`.

| Lot | État | Branche | Notes |
|---|---|---|---|
| Squelette (schéma, catégories, validateur) | fait | main | |
| B1 Infra T universel | **fusionné** (main a51353d8) | feat/b1-bulles-infra | capture docs/img/bulles-imbriquees.png |
| B2 Mécaniques campagne | **fusionné** (main a51353d8) | worktree-agent-a1391ebbb72a033ca | |
| B3 Mécaniques bataille/siège/naval | **fusionné** (main a51353d8) | worktree-agent-a8fac39c8d71471d7 | supprimer `data/codex/_b3_links.md` après B4/B5 ; B6 doit relire Breteuil/Romorantin 1356 |
| B4 Bâtiments + ressources | **fusionné** (main a51353d8) | b4-batiments | conflit gabelle résolu ; cdx_jeu_nourriture → famine/population ; alias comptoir → bâtiment |
| B5 Unités, navires, techniques | **fusionné** (main a51353d8) | | |
| B6 Audit historique récent | **fusionné** (main a51353d8) | b6-audit-historique | rapport docs/histoire/audit-2026-09-25.md |

Fusion : worktree `../gp-historien-merge` (branche `integration/historien`) puis ff-only dans main.
Commits avec chemins explicites (index partagé avec d'autres sessions).

## Journal
- 2026-09-25 : vague 1 lancée, 6 agents B1-B6 en worktrees. En cas de coupure : relancer le lot depuis son `docs/wip/bN-*.md` dans sa branche worktree.

## Incohérences code ↔ interface relevées par B2 (lot correctif B7 à prévoir)
- Coût de la cour : aide F1 dit 3 % au-delà de 8 saisons ; code 20 % au-delà de 6 (`economy.rs` OPULENCE_*).
- Dette : infobulles disent « se débandent » ; code : −10 moral/saison seulement.
- Ravitaillement : infobulle cite le pays « dévasté » ; code ne regarde qu'ami/non ami.
- Deux mécontentements : panneau lit `ProvinceState.unrest`, les révoltes lisent la moyenne des classes.
- Carte du mécontentement toujours rouge (ratio non divisé par 100).
- Données non lues : piété des édits Paix de Dieu/Carême strict, vitesse de construction (Bâtisseur, Urbaniste), `recruit_time_turns`, coûts en ressources des bâtiments, `vision_army_km`/`vision_settlement_km`.

## Points ouverts relevés par B4
- Améliorations qui remplacent : boulevard d'artillerie efface château fort (−2 fortif.) ; maison des métiers efface marché (bloque apothicairerie/draperie) ; arsenal efface champ de montre.
- `enables_units` seulement affiché (6 unités exigent vraiment un bâtiment) ; piété/prestige des bâtiments non lus ; coût en pierre non prélevé ; `satisfies_classes: []` = toutes classes dans le code.
- Pour B6 : Normandie sans pierre (pierre de Caen !) ; date de la gabelle 1341 (fiche) vs 20 mars 1342 (événement, ancien style).
- `cdx_hotel_dieu.entity` passé de tech_hospital_reform à bld_hotel_dieu.

## Points ouverts relevés par B6
- Champs de jeu historiquement faux laissés (décision de design) : owner Bergerac/Aiguillon (anglaises dès 1337, en réalité 1345), Padoue, Pavie ; kind de Rye, Binche, Andechs, Vilvorde, Corte, Linz/Vienne inversées ; fortification de Vincennes ; Ronda. Détail : rapport § 9.
- Auto-lien par alias : homonymes (« Louis de Poitiers » → bataille). Prévoir exclusion (alias plus long prioritaire / liste d'exceptions).
- 18 fiches proposées (rapport § 10) : Barbavera, Charles de la Cerda, Carrare, Mastino II, Azzone Visconti, Pepoli, Achaïe, Baudouin de Trèves, Marguerite Maultasch, Cassel 1328, Dunbar, palais des papes…

## État au 25/09 (fin de vague 1)
Tous les lots B1-B6 fusionnés dans main (a51353d8). Codex : 388 fiches (231 avant), validateur 0 erreur ; pytest 429 verts ; smoke vert sauf « music playlist too short: court », dû à 36 musiques non commitées par une autre session (hors bulles).
Fusion : 17 conflits B5/B6 résolus (textes B6 + liens B2/B5), 21 alias dédoublonnés (fiche la plus précise), `_b3_links.md`/`_b4_links.md` supprimés, `cdx_galere` → `cdx_galee`, `cdx_jeu_nourriture` → famine/population, références à des jeux commerciaux retirées (capture renommée).

## Suite possible (vague 2)
- B7 correctif code ↔ UI (listes B2/B4/B5 ci-dessus) : session code, pas historien.
- B8 auto-lien : alias le plus long prioritaire + exceptions (homonymes « Louis de Poitiers »).
- B9 18 fiches historiques proposées par B6 ; relecture Breteuil/Romorantin 1356 (B3).
- ~~Décision owner/kind~~ : tranché le 25/09 — on garde l'équilibre actuel (libertés assumées, audit § 9).
- Doublon d'entity `prov_flandre` (cdx_gand, cdx_flandre_laine).
- Unités : l'encyclopédie du smoke en compte 21 au lieu de 27 (les 27 sont chargées ; cause non trouvée).

## Vague 2 (lancée le 25/09)
| Lot | État | Notes |
|---|---|---|
| B8 Auto-lien homonymes | lancé | alias le plus long prioritaire + exceptions |
| B9a Fiches historiques 1-9 (audit § 10) | lancé | + relecture Breteuil/Romorantin 1356 |
| B9b Fiches historiques 10-18 (audit § 10) | lancé | |
