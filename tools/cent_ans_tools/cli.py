"""Command-line entry point: ``uv run --project tools cent-ans ...``."""

from __future__ import annotations

import typer
from rich.console import Console
from rich.table import Table

from cent_ans_tools import blender, budget, openrouter

app = typer.Typer(help="Outils du projet Cent Ans.", no_args_is_help=True)
budget_app = typer.Typer(
    help="Suivi du budget cloud (docs/budget.md).", no_args_is_help=True
)
models_app = typer.Typer(help="Modèles d'images OpenRouter.", no_args_is_help=True)
blender_app = typer.Typer(help="Scripts Blender.", no_args_is_help=True)
app.add_typer(budget_app, name="budget")
app.add_typer(models_app, name="models")
app.add_typer(blender_app, name="blender")

console = Console()


@budget_app.command("show")
def budget_show() -> None:
    """Affiche le tableau des dépenses et le cumul."""
    ledger = budget.BudgetLedger()
    table = Table(title="Budget cloud")
    for column in ("Date", "Service", "Objet", "Estimé", "Réel", "Cumul"):
        table.add_column(column)
    for entry in ledger.entries:
        table.add_row(
            entry.date,
            entry.service,
            entry.subject,
            budget.format_amount(entry.estimated),
            budget.format_amount(entry.actual),
            budget.format_amount(entry.cumulative),
        )
    console.print(table)
    console.print(
        f"Cumul : {budget.format_amount(ledger.total())} / {budget.format_amount(ledger.cap)}"
    )


@budget_app.command("check")
def budget_check(
    amount: float = typer.Argument(..., help="Montant estimé en dollars"),
) -> None:
    """Vérifie qu'une dépense estimée reste sous le plafond (code de sortie 1 sinon)."""
    ledger = budget.BudgetLedger()
    if ledger.check(amount):
        console.print(
            f"[green]OK[/green] : {budget.format_amount(budget.to_money(amount))} tient dans le budget"
        )
    else:
        console.print(
            f"[red]REFUSÉ[/red] : cumul {budget.format_amount(ledger.total())} + {amount} $ > plafond"
        )
        raise typer.Exit(code=1)


@models_app.command("list")
def models_list() -> None:
    """Liste les modèles OpenRouter capables de générer des images (appel gratuit)."""
    table = Table(title="Modèles d'images OpenRouter (USD)")
    table.add_column("Modèle", no_wrap=True)
    table.add_column("Prompt / token")
    table.add_column("Image / token")
    table.add_column("Estimation / image")
    for model in openrouter.list_image_models():
        price = model.estimated_price_per_image()
        table.add_row(
            model.id,
            f"{model.prompt:f}" if model.prompt is not None else "—",
            f"{model.image_output:f}" if model.image_output is not None else "—",
            f"{price:.4f} $" if price is not None else "—",
        )
    console.print(table)


@blender_app.command("smoke")
def blender_smoke() -> None:
    """Lance le script Blender de fumée (cube -> glTF temporaire)."""
    output = blender.smoke()
    if "OK" not in output.splitlines():
        console.print("[red]FAIL[/red] : le script Blender n'a pas imprimé OK")
        raise typer.Exit(code=1)
    console.print("[green]OK[/green] : cube exporté en glTF")


if __name__ == "__main__":
    app()
