#!/usr/bin/env python3
"""
database.db を作成し、user テーブルにダミーフラグを INSERT する。
Dockerfile の RUN で呼ばれる。
"""
import sqlite3, os

DB_PATH = "/var/www/html/database.db"

con = sqlite3.connect(DB_PATH)
cur = con.cursor()

# DDL
cur.executescript("""
CREATE TABLE IF NOT EXISTS user (
    id       TEXT NOT NULL PRIMARY KEY,
    password TEXT NOT NULL
);
""")

# ダミーデータ: 実際の ksnctf フラグ形式に合わせたサンプル
cur.execute(
    "INSERT OR IGNORE INTO user (id, password) VALUES (?, ?)",
    ("admin", "FLAG_LocalTestDummyFlag_ChangeMe"),
)

con.commit()
con.close()

# Apache から読めるよう権限を調整
os.chmod(DB_PATH, 0o666)
print(f"[+] {DB_PATH} created successfully.")
