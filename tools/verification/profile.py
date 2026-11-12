"""Snapshot a saved profiling workspace without benchmarking the live app store."""
import json
from contextlib import closing
from pathlib import Path
import sqlite3
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from pan_profile import snapshot_store


def catalog_rows(source):
    catalog = source / "catalog.sqlite3"
    if not catalog.is_file():
        raise ValueError(f"Profiling source has no catalog.sqlite3: {source}")
    with closing(sqlite3.connect(catalog.as_uri() + "?mode=ro", uri=True)) as connection:
        return dict(connection.execute("SELECT key, value FROM catalog"))


def saved_generation(source):
    rows = catalog_rows(source)
    if "session" not in rows:
        raise ValueError("Profiling source needs saved navigation; open and save a workspace in the app")
    if not isinstance(json.loads(rows["session"]), dict):
        raise ValueError("Profiling source session must contain saved navigation settings")
    generation = rows.get("active")
    if not generation or Path(generation).name != generation:
        raise ValueError("Profiling source needs an active published generation")
    return source / generation


def validate(source):
    database = saved_generation(source)
    with closing(sqlite3.connect(database.as_uri() + "?mode=ro", uri=True)) as connection:
        if connection.execute("SELECT COUNT(*) FROM edges").fetchone()[0] == 0:
            raise ValueError("Profiling source needs a published graph with relationships")


def main():
    source, destination = (Path(p).resolve() for p in sys.argv[1:])
    validate(source)
    snapshot_store(source, destination)
    validate(destination)
    print(f"Profiling snapshot prepared at {destination}; source remains untouched")


if __name__ == "__main__":
    main()
