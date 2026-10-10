# 0331 — Contre-déploiement de l'IA et ordre « poursuivre »

Statut : accepté.

## Contexte
Lot TW `ai-deploy` (`docs/wip/tw/bataille-ia-sieges.md` top8, `bataille-simulation.md` top8). `begin_deployment` place l'IA par rôles **avant** que le joueur place ses régiments : elle ne peut pas réagir à lui (M2 : déploiement alterné ; WH3 : placement après l'adversaire). Par ailleurs la cavalerie du joueur ne pouvait poursuivre un régiment en déroute que par un `Attack` ordinaire, sans ordre dédié ni refus clair d'une cible qui ne fuit pas.

## Décision
- **Contre-déploiement** (`sim/counter_deploy.rs`, `BattleSim::ai_counter_deploy`) : à `start_battle`, si un joueur humain commande l'autre camp et que l'IA est active, l'IA réajuste sa ligne dans sa zone (champ ouvert seulement : ni siège, ni embuscade, ni marche forcée). Trois règles, déterministes (aucun hasard) : (1) *lanciers contre cavalerie* : chaque régiment de cavalerie adverse (au plus `counter_max_pairs`) reçoit en face, par échange de place avec un fantassin du même rang, un régiment à `SpearWall` ou `PikeSquare` ; la cible est bornée à l'étendue de notre ligne de pied ; (2) *tireurs contre infanterie lente* : les tireurs se décalent latéralement (au plus `counter_shooter_shift` m) vers le centre de l'infanterie adverse de vitesse ≤ `counter_slow_speed` ; (3) *flanc refusé* : si le front adverse dépasse `counter_wide_ratio` fois le nôtre, le flanc débordé le plus fort recule de `counter_flank_pull` m et se resserre de `counter_flank_tuck` m (régiments à moins de `counter_flank_span` m de ce bord, hors général et tireurs). Puis passe anti-eau (F5d). Les valeurs vivent dans `data/rules/battle_ai.json` (schéma mis à jour).
- **Ordre `Command::Pursue { units, target, queue }`** : attaque au pas de course qui n'accepte qu'une cible ennemie présente en `Routing` (sinon `CommandError::NotRouting`, « n'est pas en déroute »). Additif (rejeux inchangés), exposé tel quel au pont par `issue_command({"type": "pursue", ...})`. Le « cap » de poursuite (`pursuit_leash`) reste propre à l'IA : le joueur garde la main sur sa cavalerie.
- Godot : raccourci P (« poursuivre ») dans `battle_hotkeys.gd`/`battle_input.gd` : chaque régiment choisi vise le fuyard ennemi le plus proche (une commande par cible). Pas de bouton dans la barre d'ordres (icône à produire).

## Conséquences
- Le joueur est puni de placer sa cavalerie sur une aile sans lanciers adverses en face, et récompensé de déborder en largeur ; l'IA contre-déployée n'est jamais plus forte, seulement mieux placée.
- Les deux IA entre elles (bataille auto 3D sans joueur) ne se répondent pas : placement simultané.
- Reste : pas de poursuite persistante après la mort de la cible (le régiment redevient libre), pas de bouton HUD.
