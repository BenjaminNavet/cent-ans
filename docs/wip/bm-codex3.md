# bm-codex3 : état (terminé)

Fait : bulles épinglées (`codex/pinned.rs`, clic droit + T), bulles `ib:` (`bubbles.rs`, `text.rs`), bulle du décor (`codex/decor.rs`, arbres + animaux), icônes d'onglets des règles (`mod.rs::add_tab_icons`), `format_auto` dans les lignes de chronique (`log.rs::record_with`). 1174 tests, clippy propre.
Preuve : `CENT_ANS_CODEX_PROOF=cdx_chene CENT_ANS_DECOR_CURSOR=center CENT_ANS_CAMERA="2.65,48.40,60"` + `proof_codex3.script` : T épingle la chaîne (`codex_pinned.alive`), le pick du décor trouve `tree:oak` -> `cdx_chene` (`codex_decor`, 41 336 arbres, 8 tuiles), `registered_ib` 121.
Hors dossier (minimal) : `ui/kit/tooltip.rs` (`TooltipRegistry::contains`), `campaign/life/mod.rs` (`pub mod fauna`), `campaign/vegetation/mod.rs` (`TreeDecor`, `TreeHit`), chronique (`record_with`).
Restes : roches et arbres de bataille pour le décor ; parenté des bulles épinglées ; fiche Codex d'une règle `ib:rule:*` au clic ; état « lu » des liens `ib:` ; clic droit et survol réels non joués (pas de souris en script) ; contenu riche des bulles d'entités (`spec_for`) remplacé par la fiche Codex.
