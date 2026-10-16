# bm-campui6 : Tech (recherche, retrait de file), colonies de la province, vignette du rapport, lien Codex

Branche `bm/campui6`. Etat : voir la fin du fichier (mis a jour a chaque commit wip).

- Tech : recherche texte `tech:search` + `DequeueResearch` (bouton "Retirer de la file", clic sur un noeud deja en file). Test game-api `dequeue_takes_a_technology_out_of_the_queue`.
- Province : `mates.rs` (pastilles + pointilles + cartouche), clic sur une case de chantier = onglet Batiments, ligne du premier palier marquee (`focus_building`).
- Minicarte : brouillard et relief deja portes par hud3 (`bake_relief`, `paint` avec `FogOfWar`) ; seule la doc d'en-tete etait perimee.
- Rapport de saison : vignette recadree en "cover" (`ModalSpec::art`, 536x170) au lieu du `[img]`.
- Tutoriel : liens Codex (`format_auto`).

Prochaine etape : tests client, vidage, fin de lot.
