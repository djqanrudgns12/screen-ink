#!/usr/bin/env python3
"""
슉쇽(screen-ink) UI 글꼴 부분 집합(woff2) 생성 스크립트.

원본 글꼴(Freesentation 4·5·6, Paperlogy 5·6)을:
  - KS X 1001 완성형 한글 2,350자
  - i18n 카탈로그(ko.json · en.json) 및 i18n-strings.csv의 모든 글자
  - 인쇄 가능 ASCII(0x20~0x7E) · UI 문장부호·화살표·기호
합집합으로 부분 집합화하고, Paperlogy의 빈 한글 글리프 8,392개를 cmap에서 제거한다.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import pathlib
import sys
from typing import Any

from fontTools.pens.boundsPen import BoundsPen
from fontTools.subset import Options, Subsetter
from fontTools.ttLib import TTFont

DEFAULT_APP_NAME = "슉쇽"
EXTRA_SYMBOLS = "·×÷–—‘’“”•…‹›−∞←↑→↓↔↕↖↗↘↙⏎⌘⌥⇧⌃⌫⇥⎋✓●○▾▸"

FONT_SPECS = [
    {"file": "Freesentation-4Regular.woff2", "family": "Freesentation", "weight": 400, "drop_empty": False},
    {"file": "Freesentation-5Medium.woff2", "family": "Freesentation", "weight": 500, "drop_empty": False},
    {"file": "Freesentation-6SemiBold.woff2", "family": "Freesentation", "weight": 600, "drop_empty": False},
    {"file": "Paperlogy-5Medium.woff2", "family": "Paperlogy", "weight": 500, "drop_empty": True},
    {"file": "Paperlogy-6SemiBold.woff2", "family": "Paperlogy", "weight": 600, "drop_empty": True},
]

PDF_SYMBOLS = "⌥⌘⇧⌃⌫⏎↩⇥⎋←↑→↓↔↕↖↗↘↙·×÷–—‘’“”•…‹›−∞✓●○▾▸+-.,:/0123456789"
DEFAULT_SYMBOLS_FONT = pathlib.Path(
    "/Users/bodyking/Desktop/Architecture/writeessay/node_modules/pretendard/dist/public/static/alternative/Pretendard-SemiBold.ttf"
)


def get_ksx1001_hangul() -> set[str]:
    """KS X 1001 완성형 한글 2,350음절(EUC-KR 0xB0A1~0xC8FE) 집합."""
    chars: set[str] = set()
    for b1 in range(0xB0, 0xC9):
        for b2 in range(0xA1, 0xFF):
            ch = bytes([b1, b2]).decode("euc_kr")
            if 0xAC00 <= ord(ch) <= 0xD7A3:
                chars.add(ch)
    return chars


def collect_target_codepoints(
    ko_json_path: pathlib.Path,
    en_json_path: pathlib.Path | None,
    i18n_csv_path: pathlib.Path | None,
    app_name: str,
) -> set[int]:
    """부분 집합에 넣을 유니코드 코드포인트 집합을 모은다."""
    texts: list[str] = [app_name, "ShookShok", EXTRA_SYMBOLS]

    for json_path in (ko_json_path, en_json_path):
        if json_path and json_path.exists():
            data = json.loads(json_path.read_text(encoding="utf-8"))
            for v in data.values():
                if isinstance(v, str):
                    texts.append(v)

    if i18n_csv_path and i18n_csv_path.exists():
        with i18n_csv_path.open("r", encoding="utf-8-sig", newline="") as f:
            reader = csv.DictReader(f)
            for row in reader:
                text = row.get("원문", "")
                if text:
                    texts.append(text)

    cps: set[int] = set(range(0x0020, 0x007F)) | {0x00A0}
    cps |= {ord(ch) for ch in get_ksx1001_hangul()}
    for txt in texts:
        for ch in txt:
            cp = ord(ch)
            if cp >= 0x20:
                cps.add(cp)
    return cps


def read_name_metadata(tt: TTFont) -> dict[int, str]:
    """name 테이블에서 저작권/라이선스/패밀리 관련 문자열(ID 0, 13, 14)을 읽는다."""
    out: dict[int, str] = {}
    if "name" not in tt:
        return out
    for name_id in (0, 13, 14):
        rec = tt["name"].getName(name_id, 3, 1, 0x0409) or tt["name"].getName(name_id, 1, 0, 0)
        if rec is None:
            for r in tt["name"].names:
                if r.nameID == name_id:
                    rec = r
                    break
        if rec is not None:
            out[name_id] = rec.toUnicode()
    return out


def subset_font(
    src_path: pathlib.Path,
    dst_path: pathlib.Path,
    target_cps: set[int],
    drop_empty: bool,
    flavor: str | None = "woff2",
) -> dict[str, Any]:
    """단일 폰트를 부분 집합화해 woff2 또는 TTF로 저장하고 검증 결과를 반환한다."""
    tt = TTFont(src_path)
    meta = read_name_metadata(tt)
    cmap = tt.getBestCmap() or {}
    gs = tt.getGlyphSet()

    if drop_empty:
        filtered_cps: set[int] = set()
        for cp in target_cps:
            if cp not in cmap:
                continue
            if cp in (0x0020, 0x00A0):
                filtered_cps.add(cp)
                continue
            pen = BoundsPen(gs)
            gs[cmap[cp]].draw(pen)
            if pen.bounds is not None:
                filtered_cps.add(cp)
        final_cps = filtered_cps
    else:
        final_cps = target_cps & set(cmap.keys())

    opts = Options()
    opts.flavor = flavor
    opts.layout_features = ["*"]
    opts.name_IDs = [0, 1, 2, 4, 6, 13, 14, 16, 17]
    opts.desubroutinize = False

    subsetter = Subsetter(options=opts)
    subsetter.populate(unicodes=final_cps)
    subsetter.subset(tt)
    tt.flavor = flavor
    if flavor is None and "glyf" in tt:
        # @pdf-lib/fontkit의 TTFSubset은 short loca(2바이트 배수)를 쓸 때 패딩을 추가하지 않으므로
        # 원본 TTF의 모든 글리프 오프셋을 2바이트 경계로 정렬해 글리프 경계 어긋남을 막는다.
        tt["glyf"].padding = 2

    dst_path.parent.mkdir(parents=True, exist_ok=True)
    tt.save(dst_path)

    # 저장된 파일을 다시 열어 형식(flavor)·테이블·빈 한글 글리프를 검증
    verify_tt = TTFont(dst_path)
    assert verify_tt.flavor == flavor, f"{dst_path.name}: flavor 불일치 ({verify_tt.flavor!r} != {flavor!r})"
    if flavor is None:
        assert "glyf" in verify_tt and "loca" in verify_tt, f"{dst_path.name}: TrueType glyf/loca 테이블 없음"
        odd_locs = [loc for loc in verify_tt["loca"].locations if loc % 2 != 0]
        assert not odd_locs, f"{dst_path.name}: 2바이트 정렬되지 않은 loca 오프셋 발견 ({len(odd_locs)}개)"
    verify_cmap = verify_tt.getBestCmap() or {}
    verify_gs = verify_tt.getGlyphSet()
    empty_hangul = 0
    for cp, gname in verify_cmap.items():
        if 0xAC00 <= cp <= 0xD7A3:
            pen = BoundsPen(verify_gs)
            verify_gs[gname].draw(pen)
            if pen.bounds is None:
                empty_hangul += 1

    if drop_empty:
        assert empty_hangul == 0, f"{dst_path.name}: 빈 한글 글리프 {empty_hangul}개 발견"

    raw = dst_path.read_bytes()
    sha256 = hashlib.sha256(raw).hexdigest()
    non_space_cps = sorted(cp for cp in verify_cmap if cp not in (0x0020, 0x00A0))

    return {
        "file": dst_path.name,
        "bytes": len(raw),
        "sha256": sha256,
        "glyphCount": len(verify_tt.getGlyphOrder()),
        "codepointCount": len(verify_cmap),
        "emptyHangulCount": empty_hangul,
        "nameMeta": meta,
        "nonSpaceChars": "".join(chr(cp) for cp in non_space_cps),
    }


def main() -> int:
    repo_root = pathlib.Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description="슉쇽 UI 글꼴 부분 집합(woff2) 생성")
    parser.add_argument("--fonts-dir", type=pathlib.Path, default=pathlib.Path.home() / "Desktop" / "Architecture" / "fonts")
    parser.add_argument("--ko-json", type=pathlib.Path, default=repo_root / "packages" / "ink-ui" / "src" / "lib" / "i18n" / "ko.json")
    parser.add_argument("--en-json", type=pathlib.Path, default=repo_root / "packages" / "ink-ui" / "src" / "lib" / "i18n" / "en.json")
    parser.add_argument("--i18n-csv", type=pathlib.Path, default=repo_root / "docs" / "plans" / "2026-11-27" / "research" / "i18n-strings.csv")
    parser.add_argument("--out-dir", type=pathlib.Path, default=repo_root / "packages" / "ink-ui" / "src" / "lib" / "fonts")
    parser.add_argument("--app-name", type=str, default=DEFAULT_APP_NAME)
    parser.add_argument("--symbols-font", type=pathlib.Path, default=DEFAULT_SYMBOLS_FONT)
    parser.add_argument("--skip-pdf", action="store_true", help="PDF용 TTF 부분 집합 생성을 건너뛴다")
    parser.add_argument("--pdf-only", action="store_true", help="UI woff2/manifest를 건드리지 않고 PDF용 TTF만 생성한다")
    args = parser.parse_args()

    fonts_dir = args.fonts_dir.expanduser().resolve()
    out_dir = args.out_dir.expanduser().resolve()
    target_cps = collect_target_codepoints(
        args.ko_json.expanduser().resolve(),
        args.en_json.expanduser().resolve() if args.en_json else None,
        args.i18n_csv.expanduser().resolve() if args.i18n_csv else None,
        args.app_name,
    )

    if not args.pdf_only:
        files_meta: list[dict[str, Any]] = []
        family_chars: dict[str, set[str]] = {}
        copyrights: dict[str, dict[int, str]] = {}

        for spec in FONT_SPECS:
            src = fonts_dir / spec["file"]
            if not src.exists():
                print(f"오류: 원본 글꼴 파일 없음: {src}", file=sys.stderr)
                return 1
            dst = out_dir / spec["file"]
            res = subset_font(src, dst, target_cps, spec["drop_empty"])
            files_meta.append(
                {
                    "file": res["file"],
                    "family": spec["family"],
                    "weight": spec["weight"],
                    "bytes": res["bytes"],
                    "sha256": res["sha256"],
                    "glyphCount": res["glyphCount"],
                    "codepointCount": res["codepointCount"],
                    "emptyHangulCount": res["emptyHangulCount"],
                }
            )
            chars_set = set(res["nonSpaceChars"])
            if spec["family"] not in family_chars:
                family_chars[spec["family"]] = chars_set
            else:
                family_chars[spec["family"]] &= chars_set
            if spec["family"] not in copyrights and res["nameMeta"]:
                copyrights[spec["family"]] = res["nameMeta"]

        total_bytes = sum(f["bytes"] for f in files_meta)
        manifest = {
            "schema": "screen-ink.fonts.manifest",
            "version": 1,
            "targetCodepointCount": len(target_cps),
            "totalBytes": total_bytes,
            "files": files_meta,
            "codepointsByFamily": {
                fam: "".join(sorted(chars)) for fam, chars in family_chars.items()
            },
        }
        manifest_path = out_dir / "manifest.json"
        manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

        print(f"글꼴 부분 집합 완료: {len(files_meta)}개 파일, 총 {total_bytes:,} B ({total_bytes / 1024:.1f} KiB)")
        for fam, meta in copyrights.items():
            print(f"[{fam}] nameMeta:", json.dumps(meta, ensure_ascii=False))

    if not args.skip_pdf:
        pdf_out_dir = out_dir / "pdf"
        mono_src = repo_root / "node_modules" / "@fontsource" / "ibm-plex-mono" / "files" / "ibm-plex-mono-latin-500-normal.woff2"
        sym_src = args.symbols_font.expanduser().resolve()
        mono_cps = set(range(0x0020, 0x007F)) | {0x00A0} | {ord(ch) for ch in "↑↓·×÷–—"}
        sym_cps = {0x0020} | {ord(ch) for ch in PDF_SYMBOLS}

        pdf_specs = [
            {"file": "Paperlogy-6SemiBold.ttf", "src": fonts_dir / "Paperlogy-6SemiBold.woff2", "family": "Paperlogy", "weight": 600, "drop_empty": True, "cps": target_cps},
            {"file": "Freesentation-4Regular.ttf", "src": fonts_dir / "Freesentation-4Regular.woff2", "family": "Freesentation", "weight": 400, "drop_empty": False, "cps": target_cps},
            {"file": "Freesentation-6SemiBold.ttf", "src": fonts_dir / "Freesentation-6SemiBold.woff2", "family": "Freesentation", "weight": 600, "drop_empty": False, "cps": target_cps},
            {"file": "IBMPlexMono-Medium.ttf", "src": mono_src, "family": "IBM Plex Mono", "weight": 500, "drop_empty": False, "cps": mono_cps},
            {"file": "Pretendard-Symbols.ttf", "src": sym_src, "family": "Pretendard", "weight": 600, "drop_empty": False, "cps": sym_cps},
        ]

        pdf_files_meta: list[dict[str, Any]] = []
        for pspec in pdf_specs:
            src_p = pspec["src"]
            if not src_p.exists():
                print(f"오류: PDF용 원본 글꼴 파일 없음: {src_p}", file=sys.stderr)
                return 1
            dst_p = pdf_out_dir / pspec["file"]
            pres = subset_font(src_p, dst_p, pspec["cps"], pspec["drop_empty"], flavor=None)
            pdf_files_meta.append(
                {
                    "file": pres["file"],
                    "family": pspec["family"],
                    "weight": pspec["weight"],
                    "bytes": pres["bytes"],
                    "sha256": pres["sha256"],
                    "glyphCount": pres["glyphCount"],
                    "codepointCount": pres["codepointCount"],
                    "emptyHangulCount": pres["emptyHangulCount"],
                }
            )

        pdf_total = sum(f["bytes"] for f in pdf_files_meta)
        pdf_manifest = {
            "schema": "screen-ink.pdf-fonts.manifest",
            "version": 1,
            "totalBytes": pdf_total,
            "files": pdf_files_meta,
        }
        (pdf_out_dir / "manifest.json").write_text(
            json.dumps(pdf_manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        print(f"PDF용 TTF 부분 집합 완료: {len(pdf_files_meta)}개 파일, 총 {pdf_total:,} B ({pdf_total / 1024:.1f} KiB)")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
