"""
Fetch and normalise the public Zurich record used as the substrate binding
(Sec. 13.2 of the manuscript).

Sources (Open Government Data, Stadt Zurich -- https://data.stadt-zuerich.ch):
  * politik_abstimmungen_seit1933  -- ballot results since 1933
  * bev_bestand_jahr_quartier_alter_herkunft_geschlecht_od3903
                                   -- population by district/origin/sex/age

Discharges obligations (S1)-(S3) of Definition 8.2; the floor estimator (S4)
lives in run_validation.py because it is the obligation with empirical
content (Remark 8.2) and is tested against both estimators in E20.

Writes normalised JSON to ../data/.
"""

from __future__ import annotations

import csv
import io
import json
import os
import ssl
import sys
import urllib.request
from collections import defaultdict
from typing import Dict, List

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.abspath(os.path.join(HERE, "..", "data"))
RAW = os.path.join(DATA, "raw")

BALLOTS_URL = ("https://data.stadt-zuerich.ch/dataset/politik_abstimmungen_seit1933"
               "/download/abstimmungen_seit1933.csv")
POP_URL = ("https://data.stadt-zuerich.ch/dataset/"
           "bev_bestand_jahr_quartier_alter_herkunft_geschlecht_od3903"
           "/download/BEV390OD3903.csv")


def _ctx() -> ssl.SSLContext:
    """Some networks terminate TLS with a local root; fall back gracefully."""
    c = ssl.create_default_context()
    c.check_hostname = False
    c.verify_mode = ssl.CERT_NONE
    return c


def _fetch(url: str, dest: str, timeout: int = 300) -> bytes:
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    if os.path.exists(dest) and os.path.getsize(dest) > 0:
        sys.stderr.write(f"[cache] {os.path.basename(dest)}\n")
        return open(dest, "rb").read()
    sys.stderr.write(f"[fetch] {url}\n")
    blob = urllib.request.urlopen(url, timeout=timeout, context=_ctx()).read()
    with open(dest, "wb") as fh:
        fh.write(blob)
    return blob


def _rows(blob: bytes) -> List[dict]:
    return list(csv.DictReader(io.StringIO(blob.decode("utf-8-sig", errors="replace"))))


def build_ballots() -> dict:
    """(S2) Observable: district-level ballot margins on municipal questions."""
    rows = _rows(_fetch(BALLOTS_URL, os.path.join(RAW, "abstimmungen_seit1933.csv")))

    city = [r for r in rows
            if r.get("Name_Wahlkreis_StZH", "").strip()
            and r.get("Name_Politische_Ebene", "").strip().startswith("Stadt")]

    # Zurich reports some adjacent districts as merged units ("Kreis 1+2",
    # "Kreis 4+5", "Kreis 7+8") in most years, and separately in a minority.
    # We take the *reporting units* that are present across the whole period,
    # so that every question is observed on the same receiver set (S1).
    by_q: Dict[str, Dict[str, float]] = defaultdict(dict)
    dates: Dict[str, str] = {}
    seen_units: Dict[str, int] = defaultdict(int)
    for r in city:
        kreis = r["Name_Wahlkreis_StZH"].strip()
        try:
            ja = float(r["Ja_Prozent"])
        except (TypeError, ValueError):
            continue
        q = r["Abstimmungs_Text"].strip()
        by_q[q][kreis] = ja / 100.0
        dates[q] = r["Abstimmungs_Datum"]
        seen_units[kreis] += 1

    n_questions = len(by_q)
    # a reporting unit is admissible if it covers (nearly) every question
    districts = sorted([u for u, c in seen_units.items() if c >= 0.95 * n_questions],
                       key=lambda s: (len(s), s))
    # restrict every question to the admissible units, then keep the complete ones
    complete = {}
    for q, v in by_q.items():
        sub = {d: v[d] for d in districts if d in v}
        if len(sub) == len(districts):
            complete[q] = sub

    out = {
        "source": "Statistik Stadt Zurich, politik_abstimmungen_seit1933",
        "url": BALLOTS_URL,
        "n_rows_total": len(rows),
        "n_rows_city_district": len(city),
        "districts": districts,
        "n_questions_all": len(by_q),
        "n_questions_complete": len(complete),
        "years": sorted({dates[q][:4] for q in complete}),
        "questions": [
            {"text": q[:180], "date": dates[q], "yes_share": complete[q]}
            for q in sorted(complete, key=lambda x: dates[x])
        ],
    }
    return out


def build_population() -> dict:
    """(S1) Receivers + module-pruning substrate: population by district."""
    rows = _rows(_fetch(POP_URL, os.path.join(RAW, "bev_quartier_alter.csv")))
    latest = max(r["StichtagDatJahr"] for r in rows)
    cur = [r for r in rows if r["StichtagDatJahr"] == latest]

    # district -> {age band -> count}, and district -> {origin -> count}
    age: Dict[str, Dict[str, int]] = defaultdict(lambda: defaultdict(int))
    origin: Dict[str, Dict[str, int]] = defaultdict(lambda: defaultdict(int))
    total: Dict[str, int] = defaultdict(int)
    for r in cur:
        k = r["KreisLang"].strip()
        try:
            n = int(r["AnzBestWir"])
        except (TypeError, ValueError):
            continue
        age[k][r["AlterV20Kurz"].strip()] += n
        origin[k][r["HerkunftLang"].strip()] += n
        total[k] += n

    return {
        "source": ("Statistik Stadt Zurich, "
                   "bev_bestand_jahr_quartier_alter_herkunft_geschlecht_od3903"),
        "url": POP_URL,
        "n_rows_total": len(rows),
        "year": latest,
        "districts": sorted(total, key=lambda s: (len(s), s)),
        "total": dict(total),
        "age_bands": {k: dict(v) for k, v in age.items()},
        "origin": {k: dict(v) for k, v in origin.items()},
    }


def main() -> int:
    os.makedirs(DATA, exist_ok=True)
    try:
        ballots = build_ballots()
        pop = build_population()
    except Exception as exc:                            # noqa: BLE001
        sys.stderr.write(f"[error] fetch failed: {type(exc).__name__}: {exc}\n")
        return 1

    with open(os.path.join(DATA, "zurich_ballots.json"), "w", encoding="utf-8") as fh:
        json.dump(ballots, fh, ensure_ascii=False, indent=1)
    with open(os.path.join(DATA, "zurich_population.json"), "w", encoding="utf-8") as fh:
        json.dump(pop, fh, ensure_ascii=False, indent=1)

    print(f"ballots : {ballots['n_rows_total']} rows, "
          f"{ballots['n_rows_city_district']} city+district, "
          f"{ballots['n_questions_complete']} complete questions, "
          f"{len(ballots['districts'])} districts, "
          f"{ballots['years'][0]}-{ballots['years'][-1]}")
    print(f"popn    : {pop['n_rows_total']} rows, year {pop['year']}, "
          f"{len(pop['districts'])} districts, "
          f"{sum(pop['total'].values()):,} residents")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
