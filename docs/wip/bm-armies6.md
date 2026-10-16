# bm-armies6 — lot armées 6 (client Bevy)

Branche `bm/armies6` (base 7ec9a21cf), dossiers `client/src/campaign/armies/` + `game-api` (vue `reach`).

## Fait
- **Armées perdues de vue** : mémoire partagée avec le parchemin (`parchment/extras.rs` : `ArmyMemory`, `Ghosts`, durée `fog_memory.turns` de `stance_cues.json`) ; `Seen` retient maintenant le nom du chef. `armies/memory.rs` : pastille grise « ✕ 1 200 » à la dernière position connue en vue 3D (fondue par le parchemin, qui a ses propres jetons) et bulle de survol « Dernière position connue — Ost de … / Vue il y a N saisons » (`bubbles.rs`, cible `ghost:<id>`, rayon 22 px).
- **Liste des armées** (`roster.rs`) : carte à deux niveaux comme Godot : sceau de cire du chef (portrait vivant, sinon armes de la faction, sceau vide `?` sans chef), pastilles d'état et d'alerte avec infobulles (icônes `hud_*`), effectif / moral / vivres, jauge de portée, lieu ; Tab / Maj+Tab (`campaign_next_idle` / `campaign_prev_idle`) parcourent les osts de la liste ouverte (`camera/focus.rs` laisse la touche à la liste).
- **Voiles héraldiques** (`sail.rs`, `shaders/sail.wesl`) : matériau « Sail » des cogues remplacé par `ExtendedMaterial<StandardMaterial, SailExt>` (laizes, armes `heraldry/<faction>.png`, creux gonflé par le vent) ; un matériau par (faction, couleur).
- **Masque de la bulle d'atteinte** (`bubble.rs`, `shaders/bubble.wesl`) : une grille posée sur le relief + texture de masque (`game_api::reach_mask`, copie de `get_reachable_area`, mips CPU) ; 9 prises adoucies, seuil 0,5, liseré, ombre d'encre, voile renforcé vers le bord, second ton pour le tour suivant.

## Restes
- Contrôle visuel (aucune capture) : voiles, bulle, sceaux, pastilles fantômes.
- Les pastilles fantômes sont en UI projetée (taille fixe), pas en `Label3D` à taille dépendante de la distance.
- Pictogrammes `hud_infantry` absents du jeu d'icônes ink : lettres de repli (`·`, `▾`).
- Bulle non suspendue pendant la fin de tour / le rejeu IA (comme avant ce lot).

## Preuve (16/10)
`proof_armies6.txt` (3 tours, hors écran, muet) : tour 0 liste ouverte 1 ost / 1 sceau portrait, Tab -> `army_0052` sélectionnée, bulle 8 448 quads, masque 638x699 texels, 10 niveaux de mips, aucune erreur GPU ; tours 1-3 sans panique. Aucune armée hostile en vue sur ces 3 tours (0 fantôme) et aucune flotte en mer (0 voile) : fantômes couverts par test (`memory.rs`, `parchment/extras.rs`), voiles non exercées en jeu. `cargo test --features dev` 1134 ok, `cargo test -p game-api` 73 ok, clippy propre.
Hors dossier : `parchment/extras.rs` (`Seen.general_name`), `camera/focus.rs` (Tab cédé à la liste), `core/crates/game-api` (vue `reach`).
