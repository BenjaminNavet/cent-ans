# TW — rapprocher Cent Ans de Total War (nuit 10-10)

Demande joueur (10-10) : orchestrateur Opus, critiques Sonnet qui comparent le jeu à Total War et listent les écarts, puis corrections en autonomie toute la nuit.
Mandat : **tout le jeu** (campagne + batailles 3D) ; références **Medieval II + Warhammer III** ; crédits cloud **≤ 5 $** (docs/budget.md) ; fusion dans main **et push autorisés** après tests verts ; pas de 3D navale.
Sessions parallèles : UX5 (5 écrans, `docs/wip/ux5-ecrans.md`), WR (restes WH, `docs/wip/wr-restes-wh.md`, ADR 030x) : ne pas toucher leurs lots. ADR TW à partir de **0320**.

## État
- [x] Vague critique (6 Sonnet ; brief `docs/wip/tw/brief-critique.md`) → `docs/wip/tw/<rôle>.md`
  rôles : bataille-controles, bataille-simulation, bataille-ia-sieges, bataille-ressenti, campagne-m2, transitions
- [x] Synthèse + lots → `docs/wip/tw/lots.md`
- [x] Vagues de corrections : 17 lots en 4 intégrations (ADR 0320-0332, 0334 ; 0333 libre)
- [x] Lot oeil : `game/tests/tw_shot.gd` (6 écrans, via `tools/godot_bg.sh`) ; corrigé : thème parchemin du briefing et de l'écran de résultat, marge du briefing
- [x] Vérif finale (fmt/clippy, 1762 tests Rust, pytest, smoke + tests Godot TW) + push

## Clôture (10-10)
Intégrations : 1 f02e5e743 (bsim, pursuit, retreat, m2a, m2b) ; 2 902d8f698 (ai-deploy, bctrl, bfeel) ;
3 82d0472bf (siege, trans, misc-ui, reinf, misc-camp) ; 4 (balance, polish, ai-camp, wardiag, oeil). Crédits cloud : 0 $.

## Restes
- Voix manquantes : `fr/en_hold_01`, `formation_01`, `retreat_01`, `rally_01/02`, `flanked_01/02` (plafond du corpus de barks relevé à 44).
- Équilibrage : taux de poursuite ; archers trop forts en duel contre l'infanterie, hobelars faibles, matrice non normalisée par le coût (`sim-battle/tests/combat/tw_balance.rs`) ; seuil de prestige 150 ; faillites élevées (89-125 / 120 tours, antérieur).
- Siège : secours absent de la résolution automatique et de la prévision ; second point d'assaut vérifié seulement au labo.
- Nouvelle partie : pas de bouton courte/longue (pont `set_victory_length` prêt).
- IA : marche forcée flamande refusée (graine 4, tour 20) ; choix de l'héritier sans âge, traits ni légitimité.
- Écran de résultat en résolution automatique : grand vide sous les cartes (mise en page partagée avec la bataille jouée).
- Partie pilote joueur : rien n'a été vu par le joueur.
