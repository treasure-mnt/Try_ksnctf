# ksnctf #35 Simple Auth 2 — ローカル再現環境

## ファイル構成

```
ksnctf35/
├── Dockerfile      # PHP 8.2 + Apache + SQLite 環境
├── auth.php        # 問題のPHPソース（そのまま）
├── init_db.py      # DDL作成 & ダミーフラグ INSERT
└── README.md
```

## 起動方法

```bash
# イメージをビルド（init_db.py が自動実行されDBが作られる）
docker build -t ksnctf35 .

# コンテナを起動
docker run -p 8080:80 ksnctf35
```

ブラウザで http://localhost:8080/auth.php を開く。

## 攻略確認

### ① ログインフォームから（正攻法）
- ID: `admin`
- Password: `FLAG_LocalTestDummyFlag_ChangeMe`

### ② DBファイルを直接ダウンロード（本問の脆弱性）
```bash
curl http://localhost:8080/database.db -o downloaded.db
sqlite3 downloaded.db "SELECT id, password FROM user;"
```

## DBの中身を変えたい場合

`init_db.py` の INSERT 行を編集してから再ビルド：

```python
cur.execute(
    "INSERT OR IGNORE INTO user (id, password) VALUES (?, ?)",
    ("admin", "FLAG_YourCustomFlag_Here"),
)
```

## Rustソルバーから接続する場合

コンテナ起動後、notebook の URL を以下に変更：

```rust
let url = "http://localhost:8080/database.db";
```
