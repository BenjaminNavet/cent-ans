# Q4 — corrections après la recette Q3

Branche : `worktree-agent-a8324042adc365e4d` (Q3 + main fusionnés). Source : `docs/audit/q3-recette.md`.
Captures : `docs/audit/captures/q4/`. Pilote : `game/tests/q3_playtest.gd` (settings.cfg sauvegardé
dans `/private/tmp/claude-501/q4_settings_backup.cfg`, à restaurer à la fin).

| # | Défaut | État | Commit |
|---|---|---|---|
| 1 | P1 bulle du conseiller (clics, modales, place) | fait, vérifié (3 unités recrutées à Paris pendant qu'il parle ; bulle absente de l'avant-bataille, de la fin, du rapport ; parle après le discours) | 43be1ab3 |
| 2 | P1 ordre des calques | fait (`PanelStack` : doc + `Tier` + `restack`, `BLOCKING_GROUP`) | 43be1ab3 |
| 3 | P1 avant-bataille navale : camp du joueur, `win_chance` | cœur correct (test Rust `q4_naval_player_side.rs`) ; la capture nv-006 montre bien la France à gauche (80 %) ; la défaite venait du joueur passif en 3D | 7d160027 |
| 4 | P2 siège 3D : brouillard vs météo, caméra | fait : l'environnement de la carte restait actif pendant les batailles (brouillard de la carte) ; caméra d'assaut face à la porte | 0323ae7b |
| 5 | P2 Haute ≈ Ultra | en cours : mesure `--map-ab=40` (lien `data/map/pyramid` vers le dépôt principal, non suivi) | |

## Prochaine étape
Mesures `--journey --map-ab=40` à 1920×1080 ; proposition chiffrée ; puis smoke, pytest, merge main.
