use std::io::{Read, Write};
use std::net::TcpStream;

// LinuxのC標準ライブラリから本物の乱数関数をインポート
unsafe extern "C" {
    fn srandom(seed: libc::c_uint);
    fn rand() -> libc::c_int;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Dockerコンテナ（ローカル）へ接続
    let host = "127.0.0.1:10777";
    println!("Connecting to {}...", host);

    // 接続した「今」の時刻をそのままシード値にする（ズレは0秒になります）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as u32;

    // 日本時間(手元)から13時間（46800秒）引いて、コンテナのニューヨーク時間にシンクロさせる！
    let server_time = now - 46800;
    
    unsafe {
        srandom(server_time);
    }

    unsafe {
        srandom(now);
    }

    let mut stream = TcpStream::connect(host)?;
    let mut buffer = [0; 4096];

    // 最初のステージ（Level 1）の正解を計算
    // 本物のespバイナリの仕様（rand() % 10）を適用
    let mut current_target = unsafe { rand() } % 10;

    loop {
        let n = stream.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        let response = String::from_utf8_lossy(&buffer[..n]);
        print!("{}", response);

        if response.contains("FLAG_") {
            println!("\n[+] Flag found!");
            break;
        }

        // ステージをクリアしたら、次の乱数を計算して更新
        if response.contains("Correct!") {
            current_target = unsafe { rand() } % 10;
        }

        if response.contains('?') {
            let answer = format!("{}\n", current_target);
            print!("-> Sending: {}", answer);
            stream.write_all(answer.as_bytes())?;
        }
    }

    Ok(())
}
