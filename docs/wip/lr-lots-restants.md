# LR — lots restants (03/10)

Mandat joueur (03/10) : faire les restes des chantiers fusionnés avec des agents (budget 20), sans dépense de crédits (pas de fal.ai, OpenRouter, ElevenLabs).
Inventaire : restes des notes wip (TB, FA, HC, OMR, EQ6, FE, TW2, VN, Q6, Q8, NT, CB, FK, JR, HV, GC, VT, SS, IA).

Exclus : RV et EM (autres sessions), FPS carte (autre session), bancs « machine calme », PC Windows, décisions joueur (TB7, GC nettoyage 1:1, forêts < 70 %, Bretagne jouable…), tout ce qui demande des crédits (147 vignettes de factions, vignettes HV, NB, GA3 fal, mocap payante).

Méthode : un worktree `../gp-lr-<id>` par lot (branche `feat/lr-<id>`, pyramide en lien symbolique, dylib copiée), note `docs/wip/lr-<id>.md` par lot ; l'orchestrateur relit, fusionne dans un worktree dédié puis `--ff-only` dans main.
Disque : `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0` dans les worktrees ; targets supprimés après fusion.

## Lots

| Lot | Contenu | Vague | État |
|---|---|---|---|
| 01 | HV : test de déclenchement des 18 événements, Lorraine `building_regions`, Brzeg/Weissenstein/Racibórz, lis de Bosnie | 1 | fait |
| 02 | FK : fiches codex des 15 événements, bateliers seulement près d'une rivière, clic sur la scène ouvre l'incident | 1 | fait |
| 03 | Nettoyages : ligne `ink-icons` dans `budget.md`, `Rect2` négatif (`_declutter_step`), code mort VT, export sans les PNG de relief inutiles | 1 | fait |
| 04 | FE : économies faibles (Irlande, Îles, Luna, Urbino), `g4.rs` ignoré, `m3_grid_ai` 50 tours | 1 | fait |
| 05 | OMR : Brandebourg sans province, Brabançons/archers écossais hors bande, Bourgogne éliminée | 1 | fait |
| 06 | Tests rouges anciens (pytest + Godot), reprise de `fix/ui-tests` | 1 | fait |
| 07 | EQ6 : révoltes (poids des garnisons dans l'ordre public), plafond du bonus de mariage | 2 | fait |
| 08 | TW2 : recrutement grisé en ruines, marqueur de ruine, surcoût mercenaire, brèche ADR 0108 | 2 | fait |
| 09 | UI 720 : en-tête du panneau de province, avis trop longs, panneau des techs | 2 | fait |
| 10 | HC lacs historiques + SS 5 réservoirs modernes | 2 | fait |
| 11 | IA : levée de siège au tour 1 par une armée de secours, armées oisives | 2 | fait |
| 12 | VN tours de siège trop grandes, NT4 ordre « tenir » côté IA, NT2 technologies du roster | 2 | fait |
| 13 | Q8 : prévision d'auto-résolution trop pessimiste | 3 | fait |
| 14 | TB : colonnes de fumée, charrette de peste trop grande | 3 | fait |
| 15 | JR : déficit structurel (Marinides, Hafsides, Lituanie, Serbie), capitale sans ville | 3 | fait |
| 16 | CB/NT : cris d'alerte en double, trait des piques plantées, transitions de rôle brusques | 3 | fait |
| 17 | HV : effet d'événement (vente de l'Estonie 1346, Algirdas 1345, Ösel-Wiek) | 3 | fait |
| 18 | FK : crue opaque, réfugiés en ligne droite | 3 | fait |

## Intégration (03/10)
Les 18 lots sont fusionnés sur `integ/lr` (worktree `../gp-lr-merge`) avec main ; worktrees des lots supprimés.
ADR numérotés à la fusion : 0177 lacs historiques (LR-10), 0178 domaine du seigneur (LR-04), 0179 lignées électives et branches cadettes (LR-05), 0180 garnison par habitant et occupation (LR-07).
`cv3_ai_stances` tourne sur les graines 1, 2 et 4 (chaque lot qui touche le flux aléatoire décalait la graine unique).

## Restes (hors LR, à arbitrer)
- Bourgogne qui disparaît après commise + traité (2 graines / 10) : 3 pistes de conception dans `lr-05.md` ; candidate : un traité ne prend pas les dernières terres d'un vassal frappé de commise.
- Brèche ADR 0108 : c'est l'assaillant qui n'exploite pas la brèche (`lr-08.md`).
- Résolution auto contre 3D : cavalerie lourde contre archers longs, égalités au défenseur (`lr-13.md`).
- Lituanie/Serbie sans armée de campagne (IA de congédiement, `lr-15.md`) ; embuscades refusées de la graine 3 (`lr-11.md`).
- Lis de Bosnie / parti angevin (générateur d'écus, `lr-01.md`) ; escaliers et ombre d'imposteur (`lr-16.md`) ; duel d'archers en bocage (`lr-11.md`).
- `test_relief_update` : publication v3 du relief interrompue (part002 + manifest manquants) ; relancer `geo relief-update` après RV.
- Partie séculaire 464 tours × 10 graines sur la branche fusionnée (LR-04/05/07/15 changent tous économie et survie).

## Prochaine étape
Vérification complète de `integ/lr` (cargo, pytest, Godot), captures de contrôle (crue, fumée, panneau 720p), puis ff dans main.
