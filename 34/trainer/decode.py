#!/usr/bin/env python3
"""
STEP4: flag.jpg (RSエンコード済み) → flag_dec.jpg (JPEG復元)
動作確認済みパラメータ: fcr=0, prim=0x11d, generator=2

使い方:
    pip install reedsolo
    python decode.py
"""
import reedsolo
import sys

INPUT  = "flag.jpg"
OUTPUT = "flag_dec.jpg"

# GF(2^8) 変換テーブル（生成多項式 0x11d）
a_table  = [0] * 256
ai_table = [0] * 256
cur = 1
for i in range(255):
    a_table[i] = cur
    cur <<= 1
    if cur & 0x100:
        cur ^= 0x11d
a_table[255] = 0
for i, v in enumerate(a_table):
    ai_table[v] = i

data = open(INPUT, 'rb').read()
n, k, ecc_len = 255, 64, 191
block_num = len(data) // n
ex_start  = block_num * k

print(f"入力: {len(data)} bytes, ブロック数: {block_num}")
print(f"訂正能力: {ecc_len//2} シンボル/ブロック ({ecc_len//2/n*100:.1f}%)")

# fcr=0 が正しいパラメータ（fcr=1だと訂正不能になる）
rsc = reedsolo.RSCodec(ecc_len, nsize=n, fcr=0, prim=0x11d, generator=2)
ans = bytearray()
err_blocks = 0
total_errata = 0

for i in range(block_num):
    d = bytes(a_table[b] for b in reversed(data[i*k:(i+1)*k]))
    e = bytes(a_table[b] for b in reversed(data[ex_start+i*ecc_len:ex_start+(i+1)*ecc_len]))
    try:
        dec, _, errata = rsc.decode(d + e)
        total_errata += len(errata)
    except Exception:
        dec = d[:k]
        err_blocks += 1
    ans += bytes(ai_table[b] for b in reversed(dec))

open(OUTPUT, 'wb').write(ans)
print(f"出力: {len(ans)} bytes → {OUTPUT}")
jpeg_ok = 'JPEG OK' if ans[:2] == b'\xff\xd8' else 'NG'
print(f"先頭4バイト: {ans[:4].hex()} ({jpeg_ok})")
print(f"訂正不能ブロック: {err_blocks} / {block_num}")
print(f"総訂正シンボル数: {total_errata} (平均 {total_errata/block_num:.1f}/ブロック)")