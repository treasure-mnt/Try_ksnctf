use reed_solomon::Decoder;

fn decode_rs_flag(
    input_path:  &str,
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    // GF(2^8) 変換テーブル（生成多項式 0x11d）
    let mut a_table  = vec![0u8; 256];
    let mut ai_table = vec![0u8; 256];
    let mut cur: u16 = 1;
    for i in 0..255usize {
        a_table[i] = cur as u8;
        cur <<= 1;
        if cur & 0x100 != 0 { cur ^= 0x11d; }
    }
    a_table[255] = 0;
    for (i, &v) in a_table.iter().enumerate() {
        ai_table[v as usize] = i as u8;
    }

    let x = std::fs::read(input_path)?;
    let n       = 255usize;
    let k       = 64usize;
    let ecc_len = n - k; // 191
    let block_num = x.len() / n;
    let ex_start  = block_num * k;

    println!("入力: {} バイト, ブロック数: {}", x.len(), block_num);

    let decoder = Decoder::new(ecc_len);
    let mut ans = Vec::with_capacity(block_num * k);

    for i in 0..block_num {
        let plain_part: Vec<u8> = x[i*k..(i+1)*k].iter().rev()
            .map(|&c| a_table[c as usize]).collect();
        let ex_part: Vec<u8> = x[ex_start + i*ecc_len..ex_start + (i+1)*ecc_len]
            .iter().rev()
            .map(|&c| a_table[c as usize]).collect();

        let mut block = plain_part;
        block.extend(ex_part);

        let corrected = match decoder.correct(&block, None) {
            Ok(c)  => c.data().to_vec(),
            Err(_) => block[0..k].to_vec(),
        };

        let mut tmp = corrected;
        tmp.reverse();
        for &c in &tmp {
            ans.push(ai_table[c as usize]);
        }
    }

    std::fs::write(output_path, &ans)?;
    println!("RSデコード完了: {} → {} バイト", input_path, ans.len());
    Ok(ans.len())
}

fn main() {
    match decode_rs_flag("flag.jpg", "flag_dec.jpg") {
        Ok(n)  => println!("STEP4完了: {} バイト", n),
        Err(e) => eprintln!("エラー: {}", e),
    }
}
