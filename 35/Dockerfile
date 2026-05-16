# ──────────────────────────────────────────────────────────────
#  ksnctf #35 Simple Auth 2 — ローカル再現環境
#  ベース: php:8.2-apache (Debian Bookworm / Trixie)
# ──────────────────────────────────────────────────────────────
FROM php:8.2-apache

# ── 1. システムパッケージ ─────────────────────────────────────
#  libsqlite3-dev : pdo_sqlite のビルドに必要なヘッダ・pkg-config
#  sqlite3        : CLI（init_db.py の確認用）
#  python3        : DB初期化スクリプトの実行
RUN apt-get update && apt-get install -y \
        libsqlite3-dev \
        sqlite3 \
        python3 \
    && rm -rf /var/lib/apt/lists/*

# ── 2. PHP拡張（pdo より先に pdo_sqlite をビルド） ───────────
#  php:8.2-apache には pdo が組み込み済みのため
#  pdo_sqlite だけ追加すれば十分。
RUN docker-php-ext-install pdo_sqlite

# ── 3. Bootstrap (オフライン用ダミー) ────────────────────────
#    実際の bootstrap.min.css がなくてもフォームは動く。
#    本物を使いたい場合は事前に bootstrap.min.css を
#    このディレクトリに置いて COPY する。
RUN echo "/* bootstrap placeholder */" \
    > /var/www/html/bootstrap.min.css

# ── 4. アプリ本体を配置 ──────────────────────────────────────
COPY auth.php /var/www/html/auth.php

# ── 5. DB初期化スクリプトを配置して実行 ─────────────────────
#    init_db.py が DDL + INSERT を行い database.db を生成する。
COPY init_db.py /tmp/init_db.py
RUN python3 /tmp/init_db.py

# ── 6. Apache の DocumentRoot 権限調整 ──────────────────────
#    www-data (Apache) が database.db を読み書きできるようにする
RUN chown www-data:www-data /var/www/html/database.db

# ── 7. ポート公開 ────────────────────────────────────────────
EXPOSE 80

# Apache はベースイメージのデフォルト CMD で起動