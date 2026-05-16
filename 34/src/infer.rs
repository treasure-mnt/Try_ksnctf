use burn::{
    backend::NdArray,
    module::Module,
    nn::{
        conv::{Conv2d, Conv2dConfig},
        pool::{MaxPool2d, MaxPool2dConfig},
        Dropout, DropoutConfig,
        Linear, LinearConfig,
        PaddingConfig2d,
    },
    record::CompactRecorder,
    tensor::{activation, backend::Backend, Tensor},
};
use image::{DynamicImage, GenericImageView};
use rayon::prelude::*;
use std::{
    fs,
    sync::{Arc, atomic::{AtomicUsize, Ordering}},
};

type BInfer = NdArray;

const NUM_CLASSES: usize = 16;
const FLAT_SIZE:   usize = 14 * 14 * 64;

// ── モデル定義（train.rsと同一）──────────────────────────────────────────

#[derive(Module, Debug)]
pub struct HexCnn<B: Backend> {
    conv1:    Conv2d<B>,
    conv2:    Conv2d<B>,
    pool:     MaxPool2d,
    dropout1: Dropout,
    fc1:      Linear<B>,
    dropout2: Dropout,
    fc2:      Linear<B>,
}

impl<B: Backend> HexCnn<B> {
    pub fn new(device: &B::Device) -> Self {
        Self {
            conv1:    Conv2dConfig::new([3, 32], [3, 3])
                          .with_padding(PaddingConfig2d::Valid).init(device),
            conv2:    Conv2dConfig::new([32, 64], [3, 3])
                          .with_padding(PaddingConfig2d::Valid).init(device),
            pool:     MaxPool2dConfig::new([2, 2]).with_strides([2, 2]).init(),
            dropout1: DropoutConfig::new(0.25).init(),
            fc1:      LinearConfig::new(FLAT_SIZE, 128).init(device),
            dropout2: DropoutConfig::new(0.50).init(),
            fc2:      LinearConfig::new(128, NUM_CLASSES).init(device),
        }
    }

    pub fn forward_infer(&self, x: Tensor<B, 4>) -> Tensor<B, 2> {
        let x = activation::relu(self.conv1.forward(x));
        let x = activation::relu(self.conv2.forward(x));
        let x = self.pool.forward(x);
        let [batch, _, _, _] = x.dims();
        let x = x.reshape([batch, FLAT_SIZE]);
        let x = activation::relu(self.fc1.forward(x));
        self.fc2.forward(x)
    }
}

// ── ユーティリティ ────────────────────────────────────────────────────────

fn img_to_tensor<B: Backend>(img: &DynamicImage, device: &B::Device) -> Tensor<B, 4> {
    let rgb = img.to_rgb8();
    let mut chw = vec![0f32; 3 * 32 * 32];
    for y in 0..32usize {
        for x in 0..32usize {
            let p = rgb.get_pixel(x as u32, y as u32);
            chw[0 * 1024 + y * 32 + x] = p[0] as f32 / 255.0;
            chw[1 * 1024 + y * 32 + x] = p[1] as f32 / 255.0;
            chw[2 * 1024 + y * 32 + x] = p[2] as f32 / 255.0;
        }
    }
    Tensor::<B, 1>::from_floats(chw.as_slice(), device).reshape([1, 3, 32, 32])
}

fn argmax_cls<B: Backend>(logits: Tensor<B, 2>) -> usize {
    let vals: Vec<f32> = logits
        .flatten::<1>(0, 1)
        .into_data()
        .to_vec::<f32>()
        .unwrap();
    vals.iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

// ── STEP3 推論 ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct TileInfo {
    file_path:  String,
    tile_idx:   usize,
    global_idx: usize,
}

fn step3_inference_and_write_flag(
    src_dir:     &str,
    model_path:  &str,
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {

    // 1. PNGを16進数ファイル名順にソート
    let mut entries: Vec<_> = fs::read_dir(src_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ex| ex == "png").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| {
        let stem = e.path().file_stem().unwrap().to_string_lossy().to_string();
        u64::from_str_radix(&stem, 16).unwrap_or(0)
    });

    // 2. タイル一覧作成
    let mut tiles = Vec::new();
    let mut global_idx = 0usize;
    for entry in &entries {
        let path = entry.path();
        let img  = image::open(&path)?;
        let n_tiles = (img.dimensions().0 / 32) as usize;
        for i in 0..n_tiles {
            tiles.push(TileInfo {
                file_path:  path.to_string_lossy().to_string(),
                tile_idx:   i,
                global_idx,
            });
            global_idx += 1;
        }
    }
    let total = tiles.len();
    println!("総タイル数: {}", total);

    // 3. 並列推論（スレッドごとにモデルを独立ロード）
    let model_path_owned = model_path.to_string();
    let processed = Arc::new(AtomicUsize::new(0));

    let results: Vec<(usize, usize)> = tiles
        .into_par_iter()
        .map(|tile| {
            let device_local: <BInfer as Backend>::Device = Default::default();
            let model_local: HexCnn<BInfer> = HexCnn::new(&device_local)
                .load_file(&model_path_owned, &CompactRecorder::new(), &device_local)
                .expect("モデルロード失敗");
            let img = image::open(&tile.file_path).expect("画像読み込み失敗");
            let tile_img = img.crop_imm(tile.tile_idx as u32 * 32, 0, 32, 32);

            // 空欄タイル検出: ピクセル値の標準偏差が閾値以下なら0で補完
            let rgb = tile_img.to_rgb8();
            let pixels_f: Vec<f32> = rgb.pixels()
                .flat_map(|p| p.0.iter().map(|&v| v as f32))
                .collect();
            let mean = pixels_f.iter().sum::<f32>() / pixels_f.len() as f32;
            let variance = pixels_f.iter().map(|&v| (v - mean).powi(2)).sum::<f32>()
                / pixels_f.len() as f32;
            let cls = if variance < 50.0 {
                // 均一色（空欄）→ 0として扱う
                0
            } else {
                // rgb済みデータを直接Tensorに変換（to_rgb8()の二重呼び出し回避）
                let mut chw = vec![0f32; 3 * 32 * 32];
                for y in 0..32usize {
                    for x in 0..32usize {
                        let p = rgb.get_pixel(x as u32, y as u32);
                        chw[0 * 1024 + y * 32 + x] = p[0] as f32 / 255.0;
                        chw[1 * 1024 + y * 32 + x] = p[1] as f32 / 255.0;
                        chw[2 * 1024 + y * 32 + x] = p[2] as f32 / 255.0;
                    }
                }
                let x = Tensor::<BInfer, 1>::from_floats(chw.as_slice(), &device_local)
                    .reshape([1, 3, 32, 32]);
                argmax_cls::<BInfer>(model_local.forward_infer(x))
            };

            let cnt = processed.fetch_add(1, Ordering::Relaxed);
            if (cnt + 1) % 10000 == 0 {
                println!("推論進捗: {}/{}", cnt + 1, total);
            }
            (tile.global_idx, cls)
        })
        .collect();

    // 4. global_idxでソートして16進文字列を構築
    let mut sorted = results;
    sorted.sort_by_key(|(idx, _)| *idx);

    let hex_chars = ['0','1','2','3','4','5','6','7',
                     '8','9','a','b','c','d','e','f'];
    let hex_str: String = sorted.iter()
        .map(|(_, cls)| hex_chars[*cls])
        .collect();

    println!("16進文字列 文字数: {}", hex_str.len());
    println!("先頭64文字: {}", &hex_str[..hex_str.len().min(64)]);

    // 5. 16進→バイナリ書き出し（Bug 1修正: ImageBufferではなくfs::write）
    let padded = if hex_str.len() % 2 == 1 {
        format!("{}0", hex_str)
    } else {
        hex_str
    };
    let bytes: Vec<u8> = padded.as_bytes().chunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap_or(0))
        .collect();

    fs::write(output_path, &bytes)?;
    println!("flag.jpg 保存: {} バイト", bytes.len());
    Ok(bytes.len())
}

// ── エントリポイント ───────────────────────────────────────────────────────

fn main() {
    match step3_inference_and_write_flag("../image", "hex_cnn_model", "flag.jpg") {
        Ok(n)  => println!("STEP3完了: {} バイト", n),
        Err(e) => eprintln!("エラー: {}", e),
    }
}
