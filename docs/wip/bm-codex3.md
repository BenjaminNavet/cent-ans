# bm-codex3 : état (terminé)

Fait : bulles épinglées (`codex/pinned.rs`, clic droit + T), bulles `ib:` (`bubbles.rs`, `text.rs`), bulle du décor (`codex/decor.rs`, arbres + animaux), icônes d'onglets des règles (`mod.rs::add_tab_icons`), `format_auto` dans les lignes de chronique (`log.rs::record_with`). 1174 tests, clippy propre.
Preuve : `CENT_ANS_CODEX_PROOF=cdx_chene CENT_ANS_DECOR_CURSOR=center CENT_ANS_CAMERA="2.65,48.40,60"` + `proof_codex3.script` : T épingle la chaîne (`codex_pinned.alive`), le pick du décor trouve `tree:oak` -> `cdx_chene` (`codex_decor`, 41 336 arbres, 8 tuiles), `registered_ib` 121.
Hors dossier (minimal) : `ui/kit/tooltip.rs` (`TooltipRegistry::contains`), `campaign/life/mod.rs` (`pub mod fauna`), `campaign/vegetation/mod.rs` (`TreeDecor`, `TreeHit`), chronique (`record_with`).
Restes : roches et arbres de bataille pour le décor ; parenté des bulles épinglées ; fiche Codex d'une règle `ib:rule:*` au clic ; état « lu » des liens `ib:` ; clic droit et survol réels non joués (pas de souris en script) ; contenu riche des bulles d'entités (`spec_for`) remplacé par la fiche Codex.

## small2 (16/10)
- Arbres de bataille : `battle/decor/trees.rs::PlantedTrees` (tous les plants en monde) + `codex/decor.rs::battle_decor_hover` (même machine d'attente que la campagne, `advance`), pick par projection de l'écran, fiche `tree:<espèce>`. Rochers de bataille : aucun instance (affleurements peints par le shader du sol), donc non cueillables ; reste.
- Parenté : `PinnedBubble.parent` ; la chaîne épinglée garde ses niveaux liés, un lien épinglé (clic droit) depuis une bulle épinglée en est l'enfant, détacher une bulle détache ses descendantes (`with_descendants`, testé).
- Clic sur `ib:rule:*` : épingle la bulle de la règle (`click_rule_links`). Les règles de `tooltips.json` n'ont aucun champ `codex` ni fiche des Règles correspondante : si une vraie fiche est voulue, c'est un choix de données à faire.
