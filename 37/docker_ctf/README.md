# ksnctf #36 "Are you ESPer?" — Dockerシミュレーション

## ファイル構成

```
.
├── Dockerfile.server    # CTFサーバー (socat + esp バイナリ)
├── Dockerfile.solver    # Rust ソルバー
├── docker-compose.yml   # 両方をまとめて起動
├── esp                  # ★問題バイナリ (要配置)
├── solver/
│   ├── Cargo.toml
│   └── src/main.rs      # glibc rand() 再実装 + 自動回答
└── README.md
```

## クイックスタート

```bash
# 1. espバイナリをこのディレクトリに置く（すでにある場合はスキップ）

# 2. サーバーを起動
docker compose up -d server

# 3. ソルバーを実行
docker compose run --rm solver

# 4. 後片付け
docker compose down
```

## 手動接続で遊ぶ

```bash
docker compose up -d server
nc localhost 10036
```

## 解法の仕組み

### バイナリ解析

```c
// esp のメインロジック (逆アセンブルから復元)
srand(time(NULL));  // ← 時刻でシード

int levels[] = {10,8,6,4,2,1,1,...,1};  // 20レベル合計45問
for (int lv = 0; lv < 20; lv++) {
    for (int ch = 0; ch < levels[lv]; ch++) {
        int answer = rand() % 10;  // ← 0〜9の正解
        printf("Level %d/20, Challenge %d/%d\n", lv+1, 20, ch+1, levels[lv]);
        int guess;
        scanf("%d", &guess);
        if (guess == answer) puts("Correct!");
        else if (guess < answer) puts("Too small");
        else puts("Too large");
    }
}
// 全問正解で /home/q36/flag.txt を表示
```

### 攻撃手法

1. **`srand(time(NULL))` の予測**  
   接続時刻を Unix time で推測し、同じシードで `rand()` シーケンスを再現する

2. **glibc `rand()` の Rust 再実装**  
   glibc の type-3 (31要素フィボナッチ LFSR) を完全に再実装。  
   C の `rand()` と完全一致することを検証済み。

3. **±60秒のシードトライ**  
   サーバーとの時刻差を吸収するため、現在時刻 ±60秒 のシードを順番に試す。

### glibc rand() 再実装 (Rust)

```rust
struct GlibcRand { table: [i32; 31], fptr: usize, rptr: usize }

impl GlibcRand {
    fn new(seed: u32) -> Self {
        // 線形合同法で初期テーブルを埋め、310回ウォームアップ
        ...
    }
    fn rand(&mut self) -> i32 {
        let result = self.table[self.fptr].wrapping_add(self.table[self.rptr]);
        self.table[self.fptr] = result;
        let val = ((result as u32) >> 1) as i32;
        self.fptr = (self.fptr + 1) % 31;
        self.rptr = (self.rptr + 1) % 31;
        val
    }
}
```

## 本番サーバーへの接続（復活した場合）

```bash
# ソルバーを本番サーバーに向ける
docker compose run --rm solver ctfq.u1tramarine.blue:10036

# または SSL 経由
# ncat --ssl ctfq.u1tramarine.blue 20036
```
