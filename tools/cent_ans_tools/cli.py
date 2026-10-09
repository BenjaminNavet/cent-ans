"""Command-line entry point: ``uv run --project tools cent-ans ...``.

Les commandes vivent dans ``cent_ans_tools/commands/`` (un module par domaine) ;
ce fichier n'assemble que le parseur racine et enregistre les sous-groupes.
L'ordre d'import fixe l'ordre d'enregistrement, donc celui de ``--help``.
"""

from __future__ import annotations

import typer

from cent_ans_tools.commands import art as art_commands
from cent_ans_tools.commands import (
    assets_materials,  # noqa: F401  (enregistre ses commandes)
    assets_media,
    budget,
    data,
    geo_map,
    geo_relief,  # noqa: F401  (enregistre ses commandes)
)

app = typer.Typer(help="Outils du projet Cent Ans.", no_args_is_help=True)
app.registered_commands.extend(data.root_commands.registered_commands)
app.add_typer(budget.budget_app, name="budget")
app.add_typer(budget.models_app, name="models")
app.add_typer(budget.blender_app, name="blender")
app.add_typer(geo_map.geo_app, name="geo")
app.add_typer(assets_media.assets_app, name="assets")
app.add_typer(art_commands.art_app, name="art")

if __name__ == "__main__":
    app()
