"""Tests for the PO before/after board (cent_ans_tools.po_board)."""

from pathlib import Path

from PIL import Image

from cent_ans_tools.po_board import GUTTER, LABEL_HEIGHT, build_board


def test_board_pairs_common_views(tmp_path: Path) -> None:
    """Only views present in both folders are paired, one row each."""
    before, after = tmp_path / "avant", tmp_path / "apres"
    before.mkdir()
    after.mkdir()
    for folder in (before, after):
        Image.new("RGB", (160, 90), "red").save(folder / "01-titre.jpg")
    Image.new("RGB", (160, 90), "blue").save(before / "02-seul.jpg")
    board = build_board(before, after, width=80)
    assert board.width == 2 * 80 + 3 * GUTTER
    assert board.height == LABEL_HEIGHT + 45 + 2 * GUTTER
