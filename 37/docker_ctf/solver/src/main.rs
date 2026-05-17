// ksnctf #36 "Are you ESPer?" - Rustソルバー
//
// 【問題】
//   nc で接続すると「0〜9の数当てゲーム」が始まる。
//   Level 1〜20、合計45問を全問一発正解するとflagが表示される。
//   Level6以降はチャンスが1回しかないので、乱数を予測する必要がある。
//
// 【解法】
//   バイナリは srand(time(NULL)) で初期化後、rand() % 10 を正解にしている。
//   接続時刻を予測して glibc rand() を Rust で再実装し、全答えを事前計算する。
//   ±60秒のシードを試してサーバー時刻のズレを吸収する。

use std::net::TcpStream;
use std::io::{Write, BufRead, BufReader};
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================
// glibc rand() の完全再実装 (type 3: 31要素 Fibonacci LFSR)
// ============================================================
struct GlibcRand {
    table: [i32; 31],
    fptr: usize,
    rptr: usize,
}

impl GlibcRand {
    fn new(seed: u32) -> Self {
        let mut table = [0i32; 31];
        table[0] = seed as i32;
        let mut kc = seed as i32;
        // 線形合同法で初期テーブルを埋める
        for i in 1..31 {
            let hi = kc / 127773;
            let lo = kc % 127773;
            kc = 16807 * lo - 2836 * hi;
            if kc < 0 {
                kc += 0x7fffffff;
            }
            table[i] = kc;
        }
        let mut rng = GlibcRand { table, fptr: 3, rptr: 0 };
        // ウォームアップ: 310回回す（glibcと同じ）
        for _ in 0..310 {
            rng.next_raw();
        }
        rng
    }

    fn next_raw(&mut self) -> i32 {
        let result = self.table[self.fptr].wrapping_add(self.table[self.rptr]);
        self.table[self.fptr] = result;
        let val = ((result as u32) >> 1) as i32;
        self.fptr = (self.fptr + 1) % 31;
        self.rptr = (self.rptr + 1) % 31;
        val
    }

    /// rand() と同等
    fn rand(&mut self) -> i32 {
        self.next_raw()
    }
}

// ============================================================
// Level構成: 合計45問
// ============================================================
const LEVELS: [usize; 20] = [10, 8, 6, 4, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];

fn generate_answers(seed: u32) -> Vec<i32> {
    let mut rng = GlibcRand::new(seed);
    let mut answers = Vec::new();
    for &count in &LEVELS {
        for _ in 0..count {
            answers.push(rng.rand() % 10);
        }
    }
    answers
}

// ============================================================
// サーバーへの接続と自動回答
// ============================================================
fn try_solve(addr: &str, seed: u32, verbose: bool) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let answers = generate_answers(seed);
    let stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;

    let mut writer = stream.try_clone()?;
    let reader = BufReader::new(&stream);

    let mut ans_idx = 0;
    let mut flag = String::new();
    let mut failed = false;

    for line in reader.lines() {
        let line = line?;
        if verbose { eprintln!("< {}", line); }

        // フラグ行の検出
        if line.starts_with("FLAG_") {
            flag = line.clone();
        }

        // "Bye" = 失敗
        if line == "Bye" {
            failed = true;
        }

        // "Level X/20, Challenge Y/Z" が来たら答えを送信
        if line.starts_with("Level ") {
            if ans_idx < answers.len() {
                let ans = answers[ans_idx];
                if verbose { eprintln!("> {}", ans); }
                writeln!(writer, "{}", ans)?;
                ans_idx += 1;
            }
        }
    }

    if failed || flag.is_empty() {
        Ok(None)
    } else {
        Ok(Some(flag))
    }
}

// ============================================================
// メイン: ±60秒のシードを試す
// ============================================================
fn main() {
    // 接続先（引数で変更可能）
    let args: Vec<String> = std::env::args().collect();
    let addr = if args.len() > 1 {
        args[1].clone()
    } else {
        // デフォルト: ローカルDockerサーバー
        "127.0.0.1:10036".to_string()
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32;

    println!("=== ksnctf #36 Are you ESPer? Solver ===");
    println!("接続先: {}", addr);
    println!("現在のUnixtime: {}", now);
    println!("±60秒のシードを試します...\n");

    for delta in 0i32..=60 {
        for &off in &[delta, -delta] {
            let seed = (now as i64 + off as i64) as u32;
            eprint!("seed={} (offset={:+3})... ", seed, off);

            match try_solve(&addr, seed, false) {
                Ok(Some(flag)) => {
                    eprintln!("🎉 SUCCESS!");
                    println!("\n🎉 FLAG取得: {}", flag);
                    return;
                }
                Ok(None) => {
                    eprintln!("miss (wrong answers)");
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    // 接続エラーの場合はここで終了
                    eprintln!("接続できません。サーバーが起動しているか確認してください。");
                    std::process::exit(1);
                }
            }
        }
    }

    eprintln!("\n失敗: 120秒範囲内でシードが見つかりませんでした。");
    eprintln!("サーバーの時刻差が大きい可能性があります。");
}
