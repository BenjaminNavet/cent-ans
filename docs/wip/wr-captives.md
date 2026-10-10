# WR captives — exécution des captifs (ADR 0303)

État : cœur fait (`ransom::execute_captive`, `Order::ExecuteCaptive`, règles `economy.json` `ransom.execution`, IA `ai_execution_orders`), pont (`execution` dans `get_ransoms`), tests Rust en cours.
Reste : bouton UI (ransom_panel.gd, confirmation en deux temps), test `wr_captives_ui_test.gd`, ADR 0303.

Mise à jour : tests Rust h5_h6 21/21 OK, clippy OK, ADR 0303 écrite, UI + test Godot écrits test Godot wr_captives_ui_test OK.
