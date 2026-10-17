# Lot battlesim (post-playtest 17/10)

État : défauts 1 à 5 traités, vérification en cours (tests core, clippy/test client, smoke).

1. Déploiement : `step_session` (client) n'avance plus tant que `Deployment.tried` est faux (joueur, pas de hook dev) ; `deploy::awaiting_deployment`.
2. Place prise : `SideResult.routed` faux pour `SquareHeld` et `Nightfall` (core) ; phrases dédiées dans `battle/hud/result.rs` (pas de mise en page touchée).
3. IA : `defender_sally_time` / `defender_sally_reach` (data/rules/battle_ai.json) : un défenseur non surclassé (ratio >= 0.85) que personne n'attaque contre-attaque ; `winning_duel_max_seconds` 300 -> 220. Premier choc (s) avant -> après : plaine joueur défenseur ~690-720 -> ~560-600 ; plaine joueur attaquant ~690 -> ~370 ; collines joueur attaquant jamais (refus) -> ~520-690. Sonde : `tests/ai_tactics/contact.rs`. Test de refus : défenseur à 60 %.
   Essais écartés : duel_limit/duel_time plus courts (déséquilibre ep9b symétrique).
4. Fin de bataille : rendu `shown_state` : après la fin, vainqueurs en pose de victoire, tous les autres au repos (idle).
5. Naval : une seule estimation (`win_chance`, 21 tirages au lieu de 5) pour verdict, % et barre.
