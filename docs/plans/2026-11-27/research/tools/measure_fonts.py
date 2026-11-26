#!/usr/bin/env python3
"""
슉쇽(screen-ink) UI 글꼴 치수 측정 · 글자 범위 검사 · 부분 집합(woff2) 크기 측정 도구.

사용 예:
  /path/to/venv/bin/python docs/plans/2026-11-27/research/tools/measure_fonts.py \
    --fonts-dir ~/Desktop/Architecture/fonts \
    --fontsource-dir node_modules/@fontsource \
    --pretendard-dir /tmp/screen-ink-font-metrics/pretendard \
    --ko-json packages/ink-ui/src/lib/i18n/ko.json \
    --i18n-csv docs/plans/2026-11-27/research/i18n-strings.csv \
    --app-name 슉쇽 \
    --out-json docs/plans/2026-11-27/research/font-metrics.json

필요 패키지: fonttools, brotli
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import math
import pathlib
import re
import sys
from datetime import datetime, timezone
from typing import Any

from fontTools.pens.boundsPen import BoundsPen
from fontTools.subset import Options, Subsetter
from fontTools.ttLib import TTFont

HANGUL_SAMPLE = "가나다한글"
KEYCAP_SYMBOLS = "⌥⌘⇧⌃⌫⏎⇥⎋←→↑↓↔×·"
TARGET_SIZES_PX = [11, 12, 13, 15, 20]
# 크로스-OS 광학 수직 중앙 정렬 목표 라인 박스 높이(선언 font-size 대비 1.20em)
NORMALIZED_LINE_BOX_EM = 1.20
# 한·영 혼합 UI 균형 가중치(한글 65% · 라틴 x-height 35% 기하평균)
BALANCED_HANGUL_WEIGHT = 0.65
BALANCED_LATIN_WEIGHT = 0.35

DEFAULT_APP_NAME = "슉쇽"

WEIGHT_NAMES = {
    100: "Thin",
    200: "ExtraLight",
    300: "Light",
    400: "Regular",
    500: "Medium",
    600: "SemiBold",
    700: "Bold",
    800: "ExtraBold",
    900: "Black",
}


def round4(val: float | None) -> float | None:
    return None if val is None else round(val, 4)


def round2(val: float | None) -> float | None:
    return None if val is None else round(val, 2)


def round1(val: float | None) -> float | None:
    return None if val is None else round(val, 1)


def get_ksx1001_hangul() -> set[str]:
    """KS X 1001 완성형 한글 2,350음절(EUC-KR 0xB0A1~0xC8FE) 집합을 반환한다."""
    chars: set[str] = set()
    for b1 in range(0xB0, 0xC9):
        for b2 in range(0xA1, 0xFF):
            ch = bytes([b1, b2]).decode("euc_kr")
            if 0xAC00 <= ord(ch) <= 0xD7A3:
                chars.add(ch)
    return chars


def extract_used_characters(
    ko_json_path: pathlib.Path,
    i18n_csv_path: pathlib.Path,
    app_name: str,
) -> dict[str, Any]:
    """ko.json + i18n-strings.csv(원문 열) + 앱 이름에서 쓰인 글자 집합을 추출한다."""
    texts: list[str] = [app_name]
    ko_data = json.loads(ko_json_path.read_text(encoding="utf-8"))
    for v in ko_data.values():
        if isinstance(v, str):
            texts.append(v)

    csv_rows = 0
    with i18n_csv_path.open("r", encoding="utf-8-sig", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            csv_rows += 1
            text = row.get("원문", "")
            if text:
                texts.append(text)

    raw_chars = set("".join(texts))
    visible_chars = {c for c in raw_chars if not c.isspace()}
    hangul_syllables = sorted(c for c in visible_chars if 0xAC00 <= ord(c) <= 0xD7A3)
    ascii_visible = sorted(c for c in visible_chars if 0x21 <= ord(c) <= 0x7E)
    non_ascii_symbols = sorted(
        c for c in visible_chars if ord(c) > 0x7E and not (0xAC00 <= ord(c) <= 0xD7A3)
    )
    ksx1001 = get_ksx1001_hangul()
    hangul_outside_ksx1001 = [c for c in hangul_syllables if c not in ksx1001]

    return {
        "sources": {
            "ko_json_entries": len(ko_data),
            "i18n_csv_rows": csv_rows,
            "app_name": app_name,
        },
        "visible_chars": visible_chars,
        "counts": {
            "total_visible": len(visible_chars),
            "hangul_syllables": len(hangul_syllables),
            "ascii_visible": len(ascii_visible),
            "non_ascii_symbols": len(non_ascii_symbols),
            "hangul_outside_ksx1001": len(hangul_outside_ksx1001),
        },
        "non_ascii_symbols": [
            {"char": c, "codepoint": f"U+{ord(c):04X}"} for c in non_ascii_symbols
        ],
        "hangul_outside_ksx1001": hangul_outside_ksx1001,
    }


def get_glyph_bounds(tt: TTFont, glyph_name: str) -> tuple[float, float, float, float] | None:
    gs = tt.getGlyphSet()
    if glyph_name not in gs:
        return None
    pen = BoundsPen(gs)
    gs[glyph_name].draw(pen)
    return pen.bounds


def inspect_digits_tabular(tt: TTFont) -> dict[str, Any]:
    cmap = tt.getBestCmap() or {}
    hmtx = tt["hmtx"]
    upem = tt["head"].unitsPerEm
    default_widths: dict[str, int] = {}
    for d in "0123456789":
        if ord(d) in cmap:
            default_widths[d] = int(hmtx[cmap[ord(d)]][0])

    unique_default = sorted(set(default_widths.values()))
    default_is_tabular = len(default_widths) == 10 and len(unique_default) == 1

    has_tnum = False
    tnum_map: dict[str, str] = {}
    if "GSUB" in tt and tt["GSUB"].table.FeatureList and tt["GSUB"].table.LookupList:
        for fr in tt["GSUB"].table.FeatureList.FeatureRecord:
            if fr.FeatureTag == "tnum":
                has_tnum = True
                for li in fr.Feature.LookupListIndex:
                    lookup = tt["GSUB"].table.LookupList.Lookup[li]
                    for st in lookup.SubTable:
                        if hasattr(st, "mapping") and isinstance(st.mapping, dict):
                            tnum_map.update(st.mapping)

    tnum_widths: dict[str, int] | None = None
    tnum_is_tabular = False
    if has_tnum and len(default_widths) == 10:
        tnum_widths = {}
        for d in "0123456789":
            base_g = cmap[ord(d)]
            sub_g = tnum_map.get(base_g, base_g)
            tnum_widths[d] = int(hmtx[sub_g][0])
        tnum_is_tabular = len(set(tnum_widths.values())) == 1

    if default_is_tabular:
        support = "default"
    elif tnum_is_tabular:
        support = "opentype_tnum"
    else:
        support = "proportional_only"

    return {
        "support": support,
        "default_is_tabular": default_is_tabular,
        "default_widths_units": default_widths,
        "default_width_min_ratio": round4(min(default_widths.values()) / upem) if default_widths else None,
        "default_width_max_ratio": round4(max(default_widths.values()) / upem) if default_widths else None,
        "has_tnum_feature": has_tnum,
        "tnum_is_tabular": tnum_is_tabular,
        "tnum_widths_units": tnum_widths,
        "tnum_width_ratio": (
            round4(next(iter(tnum_widths.values())) / upem)
            if tnum_widths and tnum_is_tabular
            else None
        ),
    }


def measure_outline_char(tt: TTFont, ch: str) -> dict[str, Any] | None:
    cmap = tt.getBestCmap() or {}
    cp = ord(ch)
    if cp not in cmap:
        return None
    gname = cmap[cp]
    bounds = get_glyph_bounds(tt, gname)
    adv = int(tt["hmtx"][gname][0])
    if bounds is None:
        return {
            "char": ch,
            "glyph": gname,
            "empty": True,
            "advance_width": adv,
        }
    x_min, y_min, x_max, y_max = bounds
    return {
        "char": ch,
        "glyph": gname,
        "empty": False,
        "x_min": round1(x_min),
        "y_min": round1(y_min),
        "x_max": round1(x_max),
        "y_max": round1(y_max),
        "ink_width": round1(x_max - x_min),
        "ink_height": round1(y_max - y_min),
        "advance_width": adv,
    }


def measure_metrics_from_ttfont(
    tt: TTFont,
    hangul_tt: TTFont | None = None,
) -> dict[str, Any]:
    """단일 TTFont(라틴/기준)와 선택적 한글 TTFont에서 핵심 치수를 측정한다."""
    h_tt = hangul_tt or tt
    head = tt["head"]
    hhea = tt["hhea"]
    os2 = tt["OS/2"]
    upem = int(head.unitsPerEm)

    hhea_asc = int(hhea.ascent)
    hhea_desc = int(hhea.descent)
    hhea_gap = int(hhea.lineGap)
    hhea_total = hhea_asc - hhea_desc + hhea_gap

    typo_asc = int(os2.sTypoAscender)
    typo_desc = int(os2.sTypoDescender)
    typo_gap = int(os2.sTypoLineGap)
    typo_total = typo_asc - typo_desc + typo_gap
    use_typo = bool(os2.fsSelection & 0x80)

    win_asc = int(os2.usWinAscent)
    win_desc = int(os2.usWinDescent)
    win_total = win_asc + win_desc

    cap_height = int(os2.sCapHeight) if getattr(os2, "version", 0) >= 2 else None
    x_height = int(os2.sxHeight) if getattr(os2, "version", 0) >= 2 else None

    h_outline = measure_outline_char(tt, "H")
    x_outline = measure_outline_char(tt, "x")
    if cap_height is None and h_outline and not h_outline["empty"]:
        cap_height = int(round(h_outline["ink_height"]))
    if x_height is None and x_outline and not x_outline["empty"]:
        x_height = int(round(x_outline["ink_height"]))

    hangul_samples = []
    for ch in HANGUL_SAMPLE:
        m = measure_outline_char(h_tt, ch)
        if m and not m["empty"]:
            hangul_samples.append(m)

    if hangul_samples:
        h_upem = int(h_tt["head"].unitsPerEm)
        avg_h = sum(m["ink_height"] for m in hangul_samples) / len(hangul_samples)
        avg_ink_w = sum(m["ink_width"] for m in hangul_samples) / len(hangul_samples)
        avg_adv_w = sum(m["advance_width"] for m in hangul_samples) / len(hangul_samples)
        avg_y_min = sum(m["y_min"] for m in hangul_samples) / len(hangul_samples)
        avg_y_max = sum(m["y_max"] for m in hangul_samples) / len(hangul_samples)
        avg_y_center = (avg_y_min + avg_y_max) / 2.0
        hangul_metrics: dict[str, Any] | None = {
            "sample_chars": HANGUL_SAMPLE,
            "avg_ink_height": round2(avg_h),
            "avg_ink_height_ratio": round4(avg_h / h_upem),
            "avg_advance_width": round2(avg_adv_w),
            "avg_advance_width_ratio": round4(avg_adv_w / h_upem),
            "avg_ink_width": round2(avg_ink_w),
            "avg_ink_width_ratio": round4(avg_ink_w / h_upem),
            "avg_y_min": round2(avg_y_min),
            "avg_y_max": round2(avg_y_max),
            "avg_y_center_ratio": round4(avg_y_center / h_upem),
            "per_char": hangul_samples,
        }
    else:
        hangul_metrics = None

    if use_typo:
        eff_mac = {
            "source": "OS/2.typo",
            "ascent_ratio": round4(typo_asc / upem),
            "descent_ratio": round4(abs(typo_desc) / upem),
            "line_gap_ratio": round4(typo_gap / upem),
            "total_ratio": round4(typo_total / upem),
        }
        eff_win = dict(eff_mac)
    else:
        eff_mac = {
            "source": "hhea",
            "ascent_ratio": round4(hhea_asc / upem),
            "descent_ratio": round4(abs(hhea_desc) / upem),
            "line_gap_ratio": round4(hhea_gap / upem),
            "total_ratio": round4(hhea_total / upem),
        }
        eff_win = {
            "source": "OS/2.win",
            "ascent_ratio": round4(win_asc / upem),
            "descent_ratio": round4(win_desc / upem),
            "line_gap_ratio": 0.0,
            "total_ratio": round4(win_total / upem),
        }

    return {
        "units_per_em": upem,
        "hhea": {
            "ascent": hhea_asc,
            "descent": hhea_desc,
            "line_gap": hhea_gap,
            "line_height_ratio": round4(hhea_total / upem),
        },
        "os2_typo": {
            "ascent": typo_asc,
            "descent": typo_desc,
            "line_gap": typo_gap,
            "line_height_ratio": round4(typo_total / upem),
            "use_typo_metrics": use_typo,
            "fs_selection_hex": hex(os2.fsSelection),
            "os2_version": int(os2.version),
        },
        "os2_win": {
            "ascent": win_asc,
            "descent": win_desc,
            "line_height_ratio": round4(win_total / upem),
        },
        "effective_browser_default": {
            "macos": eff_mac,
            "windows": eff_win,
        },
        "cap_height": {
            "units": cap_height,
            "ratio": round4(cap_height / upem) if cap_height is not None else None,
            "ink_H_height": h_outline["ink_height"] if h_outline and not h_outline["empty"] else None,
            "ink_H_ratio": round4(h_outline["ink_height"] / upem) if h_outline and not h_outline["empty"] else None,
        },
        "x_height": {
            "units": x_height,
            "ratio": round4(x_height / upem) if x_height is not None else None,
            "ink_x_height": x_outline["ink_height"] if x_outline and not x_outline["empty"] else None,
            "ink_x_ratio": round4(x_outline["ink_height"] / upem) if x_outline and not x_outline["empty"] else None,
        },
        "hangul": hangul_metrics,
        "digits_tabular": inspect_digits_tabular(tt),
    }


def inspect_coverage_and_counts(
    font_paths: list[pathlib.Path],
    used_visible_chars: set[str],
) -> tuple[dict[str, Any], dict[str, Any]]:
    """단일 파일 또는 다중 슬라이스(@fontsource)의 글리프 수·한글 음절 수·글자 범위를 검사한다."""
    all_cps: set[int] = set()
    non_empty_cps: set[int] = set()
    total_glyphs_single: int | None = None
    sum_glyphs_slices = 0

    for p in font_paths:
        tt = TTFont(p)
        g_count = len(tt.getGlyphOrder())
        sum_glyphs_slices += g_count
        if len(font_paths) == 1:
            total_glyphs_single = g_count
        cmap = tt.getBestCmap() or {}
        gs = tt.getGlyphSet()
        for cp, gname in cmap.items():
            all_cps.add(cp)
            if cp in non_empty_cps:
                continue
            pen = BoundsPen(gs)
            gs[gname].draw(pen)
            if pen.bounds is not None:
                non_empty_cps.add(cp)

    cmap_hangul = [cp for cp in all_cps if 0xAC00 <= cp <= 0xD7A3]
    rendered_hangul = [cp for cp in cmap_hangul if cp in non_empty_cps]
    empty_hangul = [cp for cp in cmap_hangul if cp not in non_empty_cps]

    ksx1001_cps = {ord(c) for c in get_ksx1001_hangul()}
    ksx1001_rendered = len(ksx1001_cps & set(rendered_hangul))

    missing_from_cmap = []
    empty_glyphs = []
    for ch in sorted(used_visible_chars):
        cp = ord(ch)
        entry = {
            "char": ch,
            "codepoint": f"U+{cp:04X}",
            "is_hangul": 0xAC00 <= cp <= 0xD7A3,
        }
        if cp not in all_cps:
            missing_from_cmap.append(entry)
        elif cp not in non_empty_cps:
            empty_glyphs.append(entry)

    keycap_supported = [c for c in KEYCAP_SYMBOLS if ord(c) in non_empty_cps]
    keycap_missing = [c for c in KEYCAP_SYMBOLS if ord(c) not in non_empty_cps]

    counts = {
        "file_count": len(font_paths),
        "total_glyphs": total_glyphs_single if total_glyphs_single is not None else len(all_cps),
        "total_glyphs_note": (
            "single_woff2_glyph_order"
            if total_glyphs_single is not None
            else f"unique_cmap_across_{len(font_paths)}_slices (raw_slice_glyph_sum={sum_glyphs_slices})"
        ),
        "cmap_codepoints": len(all_cps),
        "cmap_hangul_syllables": len(cmap_hangul),
        "rendered_hangul_syllables": len(rendered_hangul),
        "empty_hangul_syllables": len(empty_hangul),
        "ksx1001_rendered": ksx1001_rendered,
    }
    missing_hangul_count = sum(1 for x in missing_from_cmap + empty_glyphs if x["is_hangul"])
    # 라틴 전용 글꼴(한글 0자)은 JSON 가독성을 위해 한글 누락 437자 개별 나열을 생략하고 비-한글 누락만 기록
    if len(rendered_hangul) == 0 and missing_hangul_count > 0:
        missing_listed = [x for x in missing_from_cmap if not x["is_hangul"]]
        missing_note = f"라틴 전용 글꼴: 사용 한글 {missing_hangul_count}음절 전체 미포함(한글 폴백 필수), 아래 목록은 비-한글 누락만 표기"
    else:
        missing_listed = missing_from_cmap
        missing_note = None

    coverage = {
        "checked_visible_chars": len(used_visible_chars),
        "supported_visible_chars": len(used_visible_chars) - len(missing_from_cmap) - len(empty_glyphs),
        "missing_from_cmap_count": len(missing_from_cmap),
        "empty_glyph_count": len(empty_glyphs),
        "missing_hangul_count": missing_hangul_count,
        "missing_from_cmap": missing_listed,
        "missing_from_cmap_note": missing_note,
        "empty_glyphs": empty_glyphs,
        "keycap_symbols": {
            "checked": KEYCAP_SYMBOLS,
            "supported": "".join(keycap_supported),
            "missing": "".join(keycap_missing),
        },
    }
    return counts, coverage


def subset_woff2_size(
    source_path: pathlib.Path,
    unicodes: set[int],
    drop_empty_hangul: bool = False,
) -> dict[str, int | float]:
    """fontTools.subset으로 실제 Brotli 압축 woff2 바이트 크기를 측정한다."""
    tt = TTFont(source_path)
    cmap = tt.getBestCmap() or {}
    if drop_empty_hangul:
        gs = tt.getGlyphSet()
        valid_cps = set()
        for cp in unicodes:
            if cp not in cmap:
                continue
            if 0xAC00 <= cp <= 0xD7A3:
                pen = BoundsPen(gs)
                gs[cmap[cp]].draw(pen)
                if pen.bounds is None:
                    continue
            valid_cps.add(cp)
        target_cps = valid_cps
    else:
        target_cps = unicodes & set(cmap.keys())

    opts = Options()
    opts.flavor = "woff2"
    opts.layout_features = ["*"]
    opts.name_IDs = [0, 1, 2, 4, 6, 16, 17]
    opts.desubroutinize = False
    subsetter = Subsetter(options=opts)
    subsetter.populate(unicodes=target_cps)
    subsetter.subset(tt)

    buf = io.BytesIO()
    tt.save(buf)
    size_bytes = len(buf.getvalue())
    return {
        "bytes": size_bytes,
        "kib": round1(size_bytes / 1024.0),
        "included_codepoints": len(target_cps),
        "included_glyphs": len(tt.getGlyphOrder()),
    }


def parse_fontsource_css_woff2(pkg_dir: pathlib.Path, weight: int) -> list[pathlib.Path]:
    css_path = pkg_dir / f"{weight}.css"
    css = css_path.read_text(encoding="utf-8")
    names = sorted(set(re.findall(r"files/([^)]+\.woff2)", css)))
    return [pkg_dir / "files" / n for n in names]


def build_font_face_overrides(
    size_adjust_pct: float,
    optical_center_unscaled: float,
    target_line_box_em: float = NORMALIZED_LINE_BOX_EM,
) -> dict[str, Any]:
    """
    W3C CSS Fonts 5 규격 기반 광학 수직 중앙 정렬 @font-face 오버라이드 계산.
    - 선언 font-size 기준 유효 라인박스 = target_line_box_em (기본 1.20em)
    - 유효 중심 = optical_center_unscaled * scale -> 상·하 여백 완전 대칭
    - override% = (effective_metric / scale) * 100
    """
    scale = size_adjust_pct / 100.0
    c_eff = optical_center_unscaled * scale
    half_box = target_line_box_em / 2.0
    eff_asc = c_eff + half_box
    eff_desc = half_box - c_eff
    return {
        "size_adjust_pct": round2(size_adjust_pct),
        "ascent_override_pct": round2((eff_asc / scale) * 100.0),
        "descent_override_pct": round2((eff_desc / scale) * 100.0),
        "line_gap_override_pct": 0.0,
        "effective_ascent_em": round4(eff_asc),
        "effective_descent_em": round4(eff_desc),
        "effective_line_box_em": round4(eff_asc + eff_desc),
    }


def compute_calibration(
    cand_metrics: dict[str, Any],
    ref_metrics: dict[str, Any],
    is_mono: bool = False,
) -> dict[str, Any]:
    """IBM Plex Sans KR 400 기준 보정 값·CSS @font-face 오버라이드·px 환산표를 계산한다."""
    ref_h = ref_metrics["hangul"]["avg_ink_height_ratio"]
    ref_x = ref_metrics["x_height"]["ratio"]
    ref_cap = ref_metrics["cap_height"]["ratio"]
    ref_w = ref_metrics["hangul"]["avg_advance_width_ratio"]

    cand_x = cand_metrics["x_height"]["ratio"]
    cand_cap = cand_metrics["cap_height"]["ratio"]

    latin_x_ratio = cand_x / ref_x
    cap_ratio = cand_cap / ref_cap
    size_adjust_latin = (ref_x / cand_x) * 100.0

    if not is_mono and cand_metrics["hangul"] is not None:
        cand_h = cand_metrics["hangul"]["avg_ink_height_ratio"]
        cand_w = cand_metrics["hangul"]["avg_advance_width_ratio"]
        optical_center = cand_metrics["hangul"]["avg_y_center_ratio"]
        hangul_h_ratio = cand_h / ref_h
        hangul_w_ratio = cand_w / ref_w
        size_adjust_hangul = (ref_h / cand_h) * 100.0
        # 한·영 혼합 UI 균형 단일 보정치 (한글 65% · 라틴 x-height 35% 기하평균)
        size_adjust_balanced = (
            (size_adjust_hangul ** BALANCED_HANGUL_WEIGHT)
            * (size_adjust_latin ** BALANCED_LATIN_WEIGHT)
        )
        rec_basis = "balanced_ko_en_65_35"
        rec_size_adjust = size_adjust_balanced
    else:
        optical_center = cand_cap / 2.0
        hangul_h_ratio = None
        hangul_w_ratio = None
        size_adjust_hangul = None
        size_adjust_balanced = size_adjust_latin
        rec_basis = "latin_x_height"
        rec_size_adjust = size_adjust_latin

    scale_bal = rec_size_adjust / 100.0
    scale_han = (size_adjust_hangul / 100.0) if size_adjust_hangul is not None else scale_bal
    scale_lat = size_adjust_latin / 100.0

    # 1) 광학 중앙 정렬 정규화 1.20em @font-face (권장: 균형 단일 보정치 기준)
    overrides_normalized_balanced = build_font_face_overrides(
        rec_size_adjust, optical_center, NORMALIZED_LINE_BOX_EM
    )
    # 2) 광학 중앙 정렬 정규화 1.20em @font-face (한글 높이 100% 일치 기준)
    overrides_normalized_hangul = (
        build_font_face_overrides(size_adjust_hangul, optical_center, NORMALIZED_LINE_BOX_EM)
        if size_adjust_hangul is not None
        else overrides_normalized_balanced
    )
    # 3) 기존 IBM Plex Sans KR hhea(108.5% / 41.5% = 1.50em) 완전 일치 드롭인
    ref_hhea_asc = ref_metrics["hhea"]["ascent"] / ref_metrics["units_per_em"]
    ref_hhea_desc = abs(ref_metrics["hhea"]["descent"]) / ref_metrics["units_per_em"]
    overrides_match_ref_hhea = {
        "size_adjust_pct": round2(rec_size_adjust),
        "ascent_override_pct": round2((ref_hhea_asc / scale_bal) * 100.0),
        "descent_override_pct": round2((ref_hhea_desc / scale_bal) * 100.0),
        "line_gap_override_pct": 0.0,
        "effective_ascent_em": round4(ref_hhea_asc),
        "effective_descent_em": round4(ref_hhea_desc),
        "effective_line_box_em": round4(ref_hhea_asc + ref_hhea_desc),
    }

    # 4) 고급 옵션: unicode-range 분리 @font-face (한글·라틴 각각 100% 일치, 라인박스 1.20em 통일)
    unicode_range_split = None
    if size_adjust_hangul is not None and abs(size_adjust_hangul - size_adjust_latin) >= 6.0:
        unicode_range_split = {
            "hangul_rule": build_font_face_overrides(
                size_adjust_hangul, optical_center, NORMALIZED_LINE_BOX_EM
            ),
            "latin_rule": build_font_face_overrides(
                size_adjust_latin, cand_cap / 2.0, NORMALIZED_LINE_BOX_EM
            ),
            "hangul_unicode_range": "U+AC00-D7A3, U+1100-11FF, U+3130-318F",
            "latin_unicode_range": "U+0000-00FF, U+2000-206F, U+2190-23FF",
        }

    # 글자 크기 표 (11 · 12 · 13 · 15 · 20 px) — 소수 첫째 자리
    px_table_balanced = {str(px): round1(px * scale_bal) for px in TARGET_SIZES_PX}
    px_table_hangul = {str(px): round1(px * scale_han) for px in TARGET_SIZES_PX}
    px_table_latin = {str(px): round1(px * scale_lat) for px in TARGET_SIZES_PX}

    # 보정 적용 후 실측 한글·라틴 비율
    after_balanced = {
        "hangul_height_vs_ref": round4(hangul_h_ratio * scale_bal) if hangul_h_ratio is not None else None,
        "latin_x_height_vs_ref": round4(latin_x_ratio * scale_bal),
        "cap_height_vs_ref": round4(cap_ratio * scale_bal),
    }
    after_hangul_only = {
        "hangul_height_vs_ref": round4(hangul_h_ratio * scale_han) if hangul_h_ratio is not None else None,
        "latin_x_height_vs_ref": round4(latin_x_ratio * scale_han),
        "cap_height_vs_ref": round4(cap_ratio * scale_han),
    }

    # 줄 높이 권장 (단위 없는 CSS line-height)
    # 보정 후(정규화 라인박스 1.20em + 균형 크기 보정)에는 모든 글꼴에서 일관된 스케일 사용 가능
    line_height_calibrated = {
        "11": 1.36,
        "12": 1.40,
        "13": 1.45,
        "15": 1.35,
        "20": 1.25,
    }
    # 보정 미적용(원본 100%) 시 글꼴의 실제 보이는 크기에 맞춰 조정
    raw_visual_ratio = (
        math.pow(hangul_h_ratio, BALANCED_HANGUL_WEIGHT) * math.pow(latin_x_ratio, BALANCED_LATIN_WEIGHT)
        if hangul_h_ratio is not None
        else latin_x_ratio
    )
    line_height_unscaled = {
        "11": round2(max(1.30, min(1.48, 1.36 * raw_visual_ratio))),
        "12": round2(max(1.34, min(1.50, 1.40 * raw_visual_ratio))),
        "13": round2(max(1.38, min(1.54, 1.45 * raw_visual_ratio))),
        "15": round2(max(1.30, min(1.44, 1.35 * raw_visual_ratio))),
        "20": round2(max(1.20, min(1.34, 1.25 * raw_visual_ratio))),
    }

    return {
        "reference": "IBM Plex Sans KR 400",
        "ratios_vs_ref": {
            "hangul_ink_height": round4(hangul_h_ratio),
            "latin_x_height": round4(latin_x_ratio),
            "cap_height": round4(cap_ratio),
            "hangul_advance_width": round4(hangul_w_ratio),
        },
        "size_adjust_candidates_pct": {
            "hangul_ink_height": round2(size_adjust_hangul),
            "latin_x_height": round2(size_adjust_latin),
            "balanced_ko_en": round2(size_adjust_balanced),
        },
        "recommended_basis": rec_basis,
        "recommended_scale_factor": round4(scale_bal),
        "recommended_size_adjust_pct": round2(rec_size_adjust),
        "after_adjust_vs_ref": {
            "balanced_ko_en": after_balanced,
            "hangul_only": after_hangul_only,
        },
        "css_font_face_normalized_ui": overrides_normalized_balanced,
        "css_font_face_normalized_hangul_only": overrides_normalized_hangul,
        "css_font_face_match_ref_hhea": overrides_match_ref_hhea,
        "css_font_face_unicode_range_split": unicode_range_split,
        "adjusted_px_table": {
            "recommended_balanced": px_table_balanced,
            "hangul_only_basis": px_table_hangul,
            "latin_x_height_basis": px_table_latin,
        },
        "line_height_recommended": {
            "with_size_adjust": line_height_calibrated,
            "without_size_adjust": line_height_unscaled,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="슉쇽 UI 글꼴 치수·글자범위·서브셋 크기 측정")
    parser.add_argument("--fonts-dir", type=pathlib.Path, required=True, help="Freesentation·Paperlogy woff2 폴더")
    parser.add_argument("--fontsource-dir", type=pathlib.Path, required=True, help="node_modules/@fontsource 폴더")
    parser.add_argument("--pretendard-dir", type=pathlib.Path, required=True, help="Pretendard static woff2 폴더")
    parser.add_argument("--ko-json", type=pathlib.Path, required=True, help="ko.json 경로")
    parser.add_argument("--i18n-csv", type=pathlib.Path, required=True, help="i18n-strings.csv 경로")
    parser.add_argument("--app-name", type=str, default=DEFAULT_APP_NAME, help="앱 이름 문자열")
    parser.add_argument("--out-json", type=pathlib.Path, required=True, help="결과 JSON 출력 경로")
    parser.add_argument("--skip-subset", action="store_true", help="부분 집합 woff2 크기 실측 건너뛰기")
    args = parser.parse_args()

    fonts_dir = args.fonts_dir.expanduser().resolve()
    fontsource_dir = args.fontsource_dir.expanduser().resolve()
    pretendard_dir = args.pretendard_dir.expanduser().resolve()

    used_info = extract_used_characters(args.ko_json, args.i18n_csv, args.app_name)
    used_visible = used_info["visible_chars"]

    used_only_cps = {ord(c) for c in used_visible} | {0x0020}
    ascii_printable_cps = set(range(0x0020, 0x007F))
    used_plus_ascii_cps = used_only_cps | ascii_printable_cps
    ksx1001_cps = {ord(c) for c in get_ksx1001_hangul()}
    ksx1001_plus_used_cps = ksx1001_cps | used_plus_ascii_cps

    targets: list[dict[str, Any]] = []

    sans_kr_pkg = fontsource_dir / "ibm-plex-sans-kr"
    for w in (400, 500, 600, 700):
        slices = parse_fontsource_css_woff2(sans_kr_pkg, w)
        kor_single = sans_kr_pkg / "files" / f"ibm-plex-sans-kr-korean-{w}-normal.woff2"
        lat_single = sans_kr_pkg / "files" / f"ibm-plex-sans-kr-latin-{w}-normal.woff2"
        targets.append(
            {
                "id": f"ibm-plex-sans-kr-{w}",
                "family": "IBM Plex Sans KR",
                "weight": w,
                "weight_name": WEIGHT_NAMES[w],
                "role": "baseline_sans",
                "is_mono": False,
                "metrics_latin_file": lat_single,
                "metrics_hangul_file": kor_single,
                "coverage_files": slices,
                "subset_source_file": kor_single,
                "full_bytes": sum(p.stat().st_size for p in slices),
                "full_single_korean_bytes": kor_single.stat().st_size,
                "file_label": f"@fontsource/ibm-plex-sans-kr/{w}.css ({len(slices)} woff2 slices)",
            }
        )

    mono_pkg = fontsource_dir / "ibm-plex-mono"
    for w in (400, 500):
        slices = parse_fontsource_css_woff2(mono_pkg, w)
        lat_single = mono_pkg / "files" / f"ibm-plex-mono-latin-{w}-normal.woff2"
        targets.append(
            {
                "id": f"ibm-plex-mono-{w}",
                "family": "IBM Plex Mono",
                "weight": w,
                "weight_name": WEIGHT_NAMES[w],
                "role": "baseline_mono",
                "is_mono": True,
                "metrics_latin_file": lat_single,
                "metrics_hangul_file": None,
                "coverage_files": slices,
                "subset_source_file": lat_single,
                "full_bytes": sum(p.stat().st_size for p in slices),
                "full_latin_bytes": lat_single.stat().st_size,
                "file_label": f"@fontsource/ibm-plex-mono/{w}.css ({len(slices)} woff2 slices)",
            }
        )

    for w, fname in [
        (400, "Freesentation-4Regular.woff2"),
        (500, "Freesentation-5Medium.woff2"),
        (600, "Freesentation-6SemiBold.woff2"),
        (700, "Freesentation-7Bold.woff2"),
    ]:
        p = fonts_dir / fname
        targets.append(
            {
                "id": f"freesentation-{w}",
                "family": "Freesentation",
                "weight": w,
                "weight_name": WEIGHT_NAMES[w],
                "role": "candidate_local",
                "is_mono": False,
                "metrics_latin_file": p,
                "metrics_hangul_file": p,
                "coverage_files": [p],
                "subset_source_file": p,
                "full_bytes": p.stat().st_size,
                "file_label": fname,
            }
        )

    for w, fname in [
        (500, "Paperlogy-5Medium.woff2"),
        (600, "Paperlogy-6SemiBold.woff2"),
        (700, "Paperlogy-7Bold.woff2"),
        (800, "Paperlogy-8ExtraBold.woff2"),
        (900, "Paperlogy-9Black.woff2"),
    ]:
        p = fonts_dir / fname
        targets.append(
            {
                "id": f"paperlogy-{w}",
                "family": "Paperlogy",
                "weight": w,
                "weight_name": WEIGHT_NAMES[w],
                "role": "candidate_local",
                "is_mono": False,
                "metrics_latin_file": p,
                "metrics_hangul_file": p,
                "coverage_files": [p],
                "subset_source_file": p,
                "full_bytes": p.stat().st_size,
                "file_label": fname,
                "has_empty_hangul_cleanup": True,
            }
        )

    pretendard_provenance = []
    for w, fname in [
        (400, "Pretendard-Regular.woff2"),
        (500, "Pretendard-Medium.woff2"),
        (600, "Pretendard-SemiBold.woff2"),
        (700, "Pretendard-Bold.woff2"),
    ]:
        p = pretendard_dir / fname
        raw = p.read_bytes()
        pretendard_provenance.append(
            {
                "file": fname,
                "weight": w,
                "bytes": len(raw),
                "kib": round1(len(raw) / 1024.0),
                "sha256_prefix": hashlib.sha256(raw).hexdigest()[:16],
                "official_url": f"https://raw.githubusercontent.com/orioncactus/pretendard/v1.3.9/packages/pretendard/dist/web/static/woff2/{fname}",
            }
        )
        targets.append(
            {
                "id": f"pretendard-{w}",
                "family": "Pretendard",
                "weight": w,
                "weight_name": WEIGHT_NAMES[w],
                "role": "candidate_ofl_reference",
                "is_mono": False,
                "metrics_latin_file": p,
                "metrics_hangul_file": p,
                "coverage_files": [p],
                "subset_source_file": p,
                "full_bytes": len(raw),
                "file_label": fname,
            }
        )

    measured_faces: list[dict[str, Any]] = []
    ref_metrics: dict[str, Any] | None = None

    for t in targets:
        lat_tt = TTFont(t["metrics_latin_file"])
        han_tt = TTFont(t["metrics_hangul_file"]) if t["metrics_hangul_file"] else None
        metrics = measure_metrics_from_ttfont(lat_tt, han_tt)
        counts, coverage = inspect_coverage_and_counts(t["coverage_files"], used_visible)

        subsets: dict[str, Any] | None = None
        if not args.skip_subset:
            src_p = t["subset_source_file"]
            s_used_only = subset_woff2_size(src_p, used_only_cps, drop_empty_hangul=True)
            s_used_ascii = subset_woff2_size(src_p, used_plus_ascii_cps, drop_empty_hangul=True)
            s_ksx_used = subset_woff2_size(src_p, ksx1001_plus_used_cps, drop_empty_hangul=True)
            subsets = {
                "used_only": s_used_only,
                "used_plus_ascii": s_used_ascii,
                "ksx1001_plus_used": s_ksx_used,
                "full": {
                    "bytes": t["full_bytes"],
                    "kib": round1(t["full_bytes"] / 1024.0),
                    "source": t["file_label"],
                },
            }
            if "full_single_korean_bytes" in t:
                subsets["full_single_korean_woff2"] = {
                    "bytes": t["full_single_korean_bytes"],
                    "kib": round1(t["full_single_korean_bytes"] / 1024.0),
                    "note": "ibm-plex-sans-kr-korean-{w}-normal.woff2 (11,172 한글 + ASCII 단일 파일; 보조 기호 ↔·∞ 2자 미포함)",
                }
            if "full_latin_bytes" in t:
                subsets["full_latin_woff2"] = {
                    "bytes": t["full_latin_bytes"],
                    "kib": round1(t["full_latin_bytes"] / 1024.0),
                    "note": "ibm-plex-mono-latin-{w}-normal.woff2",
                }
            if t.get("has_empty_hangul_cleanup"):
                full_tt = TTFont(src_p)
                all_non_empty = set()
                f_cmap = full_tt.getBestCmap() or {}
                f_gs = full_tt.getGlyphSet()
                for cp, gname in f_cmap.items():
                    if 0xAC00 <= cp <= 0xD7A3:
                        pen = BoundsPen(f_gs)
                        f_gs[gname].draw(pen)
                        if pen.bounds is None:
                            continue
                    all_non_empty.add(cp)
                subsets["full_cleaned_no_empty_hangul"] = subset_woff2_size(
                    src_p, all_non_empty, drop_empty_hangul=False
                )

        face_entry = {
            "id": t["id"],
            "family": t["family"],
            "weight": t["weight"],
            "weight_name": t["weight_name"],
            "role": t["role"],
            "is_mono": t["is_mono"],
            "file_label": t["file_label"],
            "metrics": metrics,
            "glyph_counts": counts,
            "coverage": coverage,
            "subset_woff2_sizes": subsets,
        }
        if t["id"] == "ibm-plex-sans-kr-400":
            ref_metrics = metrics
        measured_faces.append(face_entry)

    assert ref_metrics is not None, "기준 글꼴(ibm-plex-sans-kr-400)을 찾지 못했습니다."

    for face in measured_faces:
        face["calibration"] = compute_calibration(
            face["metrics"], ref_metrics, is_mono=face["is_mono"]
        )

    rep_ids = {
        "IBM Plex Sans KR": "ibm-plex-sans-kr-400",
        "IBM Plex Mono": "ibm-plex-mono-400",
        "Freesentation": "freesentation-400",
        "Paperlogy": "paperlogy-500",
        "Pretendard": "pretendard-400",
    }
    by_id = {f["id"]: f for f in measured_faces}
    families_summary: dict[str, Any] = {}
    for fam, rid in rep_ids.items():
        rep = by_id[rid]
        fam_faces = [f for f in measured_faces if f["family"] == fam]
        families_summary[fam] = {
            "representative_face_id": rid,
            "measured_weights": [f["weight"] for f in fam_faces],
            "units_per_em": rep["metrics"]["units_per_em"],
            "hangul_avg_ink_height_ratio": (
                rep["metrics"]["hangul"]["avg_ink_height_ratio"]
                if rep["metrics"]["hangul"]
                else None
            ),
            "hangul_avg_advance_width_ratio": (
                rep["metrics"]["hangul"]["avg_advance_width_ratio"]
                if rep["metrics"]["hangul"]
                else None
            ),
            "hangul_avg_y_center_ratio": (
                rep["metrics"]["hangul"]["avg_y_center_ratio"]
                if rep["metrics"]["hangul"]
                else None
            ),
            "x_height_ratio": rep["metrics"]["x_height"]["ratio"],
            "cap_height_ratio": rep["metrics"]["cap_height"]["ratio"],
            "digits_tabular": rep["metrics"]["digits_tabular"]["support"],
            "rendered_hangul_syllables": rep["glyph_counts"]["rendered_hangul_syllables"],
            "empty_hangul_syllables": rep["glyph_counts"]["empty_hangul_syllables"],
            "missing_from_cmap_count": rep["coverage"]["missing_from_cmap_count"],
            "empty_glyph_count": rep["coverage"]["empty_glyph_count"],
            "keycap_symbols": rep["coverage"]["keycap_symbols"],
            "calibration": rep["calibration"],
        }

    app_tokens = {
        "base_scale_px": TARGET_SIZES_PX,
        "reference_family": "IBM Plex Sans KR",
        "normalized_line_box_em": NORMALIZED_LINE_BOX_EM,
        "balanced_formula": "size_adjust_hangul^0.65 * size_adjust_latin^0.35",
        "by_family": {
            fam: {
                "font_scale": fam_info["calibration"]["recommended_scale_factor"],
                "font_scale_hangul_only": (
                    round4(fam_info["calibration"]["size_adjust_candidates_pct"]["hangul_ink_height"] / 100.0)
                    if fam_info["calibration"]["size_adjust_candidates_pct"]["hangul_ink_height"] is not None
                    else fam_info["calibration"]["recommended_scale_factor"]
                ),
                "size_adjust": f"{fam_info['calibration']['recommended_size_adjust_pct']}%",
                "font_face_normalized_ui": {
                    "size-adjust": f"{fam_info['calibration']['css_font_face_normalized_ui']['size_adjust_pct']}%",
                    "ascent-override": f"{fam_info['calibration']['css_font_face_normalized_ui']['ascent_override_pct']}%",
                    "descent-override": f"{fam_info['calibration']['css_font_face_normalized_ui']['descent_override_pct']}%",
                    "line-gap-override": f"{fam_info['calibration']['css_font_face_normalized_ui']['line_gap_override_pct']}%",
                },
                "font_face_normalized_hangul_only": {
                    "size-adjust": f"{fam_info['calibration']['css_font_face_normalized_hangul_only']['size_adjust_pct']}%",
                    "ascent-override": f"{fam_info['calibration']['css_font_face_normalized_hangul_only']['ascent_override_pct']}%",
                    "descent-override": f"{fam_info['calibration']['css_font_face_normalized_hangul_only']['descent_override_pct']}%",
                    "line-gap-override": f"{fam_info['calibration']['css_font_face_normalized_hangul_only']['line_gap_override_pct']}%",
                },
                "font_face_match_ibm_plex_hhea": {
                    "size-adjust": f"{fam_info['calibration']['css_font_face_match_ref_hhea']['size_adjust_pct']}%",
                    "ascent-override": f"{fam_info['calibration']['css_font_face_match_ref_hhea']['ascent_override_pct']}%",
                    "descent-override": f"{fam_info['calibration']['css_font_face_match_ref_hhea']['descent_override_pct']}%",
                    "line-gap-override": f"{fam_info['calibration']['css_font_face_match_ref_hhea']['line_gap_override_pct']}%",
                },
                "unicode_range_split": fam_info["calibration"]["css_font_face_unicode_range_split"],
                "adjusted_px": fam_info["calibration"]["adjusted_px_table"]["recommended_balanced"],
                "adjusted_px_hangul_only": fam_info["calibration"]["adjusted_px_table"]["hangul_only_basis"],
                "line_height": fam_info["calibration"]["line_height_recommended"]["with_size_adjust"],
                "line_height_unscaled": fam_info["calibration"]["line_height_recommended"]["without_size_adjust"],
            }
            for fam, fam_info in families_summary.items()
        },
    }

    output = {
        "schema_version": "1.1.0",
        "generated_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "measurement_spec": {
            "reference_font": "IBM Plex Sans KR 400 (Regular)",
            "hangul_sample_chars": HANGUL_SAMPLE,
            "keycap_symbols_checked": KEYCAP_SYMBOLS,
            "target_sizes_px": TARGET_SIZES_PX,
            "normalized_line_box_em": NORMALIZED_LINE_BOX_EM,
            "balanced_weights": {
                "hangul_ink_height": BALANCED_HANGUL_WEIGHT,
                "latin_x_height": BALANCED_LATIN_WEIGHT,
            },
            "subset_sets": {
                "used_only_codepoints": len(used_only_cps),
                "used_plus_ascii_codepoints": len(used_plus_ascii_cps),
                "ksx1001_plus_used_codepoints": len(ksx1001_plus_used_cps),
            },
        },
        "external_downloads_not_committed": {
            "pretendard": {
                "release": "v1.3.9",
                "license": "SIL Open Font License 1.1",
                "release_url": "https://github.com/orioncactus/pretendard/releases/tag/v1.3.9",
                "files": pretendard_provenance,
            }
        },
        "character_corpus": {
            "sources": used_info["sources"],
            "counts": used_info["counts"],
            "non_ascii_symbols": used_info["non_ascii_symbols"],
            "hangul_outside_ksx1001": used_info["hangul_outside_ksx1001"],
        },
        "families": families_summary,
        "app_tokens": app_tokens,
        "faces": measured_faces,
    }

    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(json.dumps(output, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    print(f"Wrote {args.out_json} ({len(measured_faces)} faces)")
    for fam, info in families_summary.items():
        cal = info["calibration"]
        ov = cal["css_font_face_normalized_ui"]
        px = cal["adjusted_px_table"]["recommended_balanced"]
        print(
            f"  {fam:18s} | balanced={cal['recommended_size_adjust_pct']:6.2f}% "
            f"(han={str(cal['size_adjust_candidates_pct']['hangul_ink_height']):>6s}%, lat={cal['size_adjust_candidates_pct']['latin_x_height']:6.2f}%) "
            f"| asc={ov['ascent_override_pct']:6.2f}% desc={ov['descent_override_pct']:5.2f}% "
            f"| px={px}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
