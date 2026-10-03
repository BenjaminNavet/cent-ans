# A6-L6 — fenêtres et notifications (U6 U7 U8 U10 U17 U20)

Branche `worktree-agent-ab307e092649e7ed9`. Tableau : `docs/audit/a6-audit-joueur.md`.

## État
- Cœur : `core/crates/sim-campaign/src/news_relevance.rs` (`NewsRelevance`, `Relevance`), pont `CampaignSim.classify_news(events) -> PackedStringArray` (`player`, `related`, `neighbor`, `far`). Test Rust : 2 tests unitaires.
- U6/U7 : `MapUI.ensure_relevance` / `relevance_of` / `keeps_news` lisent le cœur (repli sur `NewsInterest` si le pont est ancien) ; les nouvelles `far` vont dans l'onglet « Monde » (replié) du journal, sans lettre ; alertes filtrées (`alerts.gd`).
- U10 : `game/scripts/ui/modal_queue.gd` (`ModalQueue`), possédée par `MapUI.modal_queue` ; capture, chronique, rencontre (décisions) et rapport de saison y passent ; rapport tardif (bataille en milieu de tour) fusionné sans rouvrir ; avis suspendus pendant une modale. Test : `game/tests/a6_l6_modal_queue_test.gd`.
- U8 : barre d'agents bornée à gauche de la zone latérale, fermée par Échap (`_shortcut_input`) et à l'ouverture d'un panneau de province ou de colonie.
- U17 : registre des agents : enfants retirés avant `queue_free`, hauteur recalculée (`_fit_registry`).
- U20 : cloche désactivée (infobulle `end_turn_modal`) pendant une modale de la file ou la pause.

## Points ouverts
- Le dylib de `game/bin` doit être recompilé (`core/build.sh`) pour que `classify_news` existe ; sans lui, repli sur `NewsInterest`.
- Les nouvelles « publiques » (JR5) restent poussées à tous, par conception.
- Pas de vérification visuelle (pas d'outil de capture) : hauteur du registre et position de la barre d'agents à contrôler à l'œil.
