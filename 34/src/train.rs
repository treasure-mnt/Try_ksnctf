use burn::{
    backend::NdArray,
    module::Module,
    nn::{
        conv::{Conv2d, Conv2dConfig},
        pool::{MaxPool2d, MaxPool2dConfig},
        Dropout, DropoutConfig,
        Linear, LinearConfig,
        PaddingConfig2d,
        loss::CrossEntropyLoss,
    },
    optim::{AdamConfig, GradientsParams, Optimizer},
    record::{CompactRecorder, Recorder},
    tensor::{
        backend::{AutodiffBackend, Backend},
        activation, Tensor,
    },
};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use std::{fs, path::Path};

type B      = burn::backend::Autodiff<NdArray>;
type BInfer = NdArray;

const NUM_CLASSES: usize = 16;
const FLAT_SIZE:   usize = 14 * 14 * 64; // conv1→conv2→pool後のサイズ

// ── モデル定義 ────────────────────────────────────────────────────────────

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

    /// 学習用: Dropout あり
    pub fn forward_train(&self, x: Tensor<B, 4>) -> Tensor<B, 2> {
        let x = activation::relu(self.conv1.forward(x)); // [B,32,30,30]
        let x = activation::relu(self.conv2.forward(x)); // [B,64,28,28]
        let x = self.pool.forward(x);                    // [B,64,14,14]
        let x = self.dropout1.forward(x);
        let [batch, _, _, _] = x.dims();
        let x = x.reshape([batch, FLAT_SIZE]);
        let x = activation::relu(self.fc1.forward(x));
        let x = self.dropout2.forward(x);
        self.fc2.forward(x)
    }

    /// 推論用: Dropout なし
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

// ── データ読み込み（シングルスレッド・順序保証）──────────────────────────

fn load_dataset(
    base_dir: &str,
) -> Result<(Vec<Vec<f32>>, Vec<usize>), Box<dyn std::error::Error>> {
    let class_names = [
        "0","1","2","3","4","5","6","7",
        "8","9","a","b","c","d","e","f",
    ];
    let mut pixels_all: Vec<Vec<f32>> = Vec::new();
    let mut labels_all: Vec<usize>    = Vec::new();

    // ── ファイルリストをシングルスレッドで収集（順序保証）────────────────
    let mut file_list: Vec<(usize, std::path::PathBuf)> = Vec::new();
    for (class_idx, &cls) in class_names.iter().enumerate() {
        let class_dir = format!("{}/{}", base_dir, cls);
        if !Path::new(&class_dir).exists() {
            eprintln!("[WARN] クラスディレクトリなし: {}", class_dir);
            continue;
        }
        let mut files: Vec<_> = fs::read_dir(&class_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ex| ex == "png").unwrap_or(false))
            .collect();
        files.sort_by_key(|e| e.file_name());
        for f in files {
            file_list.push((class_idx, f.path()));
        }
    }

    // ── 並列デコード（rayon）順序はfile_listのインデックスで保証 ─────────
    let decoded: Vec<Option<Vec<f32>>> = file_list.par_iter()
        .map(|(_, path)| {
            let rgb = image::open(path).ok()?
                .resize_exact(32, 32, image::imageops::FilterType::Nearest)
                .to_rgb8();
            let mut chw = vec![0f32; 3 * 32 * 32];
            for y in 0..32usize {
                for x in 0..32usize {
                    let p = rgb.get_pixel(x as u32, y as u32);
                    chw[0 * 1024 + y * 32 + x] = p[0] as f32 / 255.0;
                    chw[1 * 1024 + y * 32 + x] = p[1] as f32 / 255.0;
                    chw[2 * 1024 + y * 32 + x] = p[2] as f32 / 255.0;
                }
            }
            Some(chw)
        })
        .collect();

    // ── 結果をfile_listと同じ順序でpixels_all/labels_allに格納 ──────────
    for (opt_px, (class_idx, _)) in decoded.into_iter().zip(file_list.iter()) {
        if let Some(px) = opt_px {
            pixels_all.push(px);
            labels_all.push(*class_idx);
        }
    }

    if pixels_all.is_empty() {
        return Err(format!(
            "サンプル数が0です。{}/{{0..f}}/ にPNGを配置してください", base_dir
        ).into());
    }
    // ここまでで pixels_all / labels_all はシングルスレッドで順序通りに収集済み
    // → labels と pixels の対応は保証されている

    // クラス別枚数を表示
    let mut counts = [0usize; 16];
    for &l in &labels_all { counts[l] += 1; }
    println!("クラス別枚数:");
    for (i, &c) in counts.iter().enumerate() {
        let name = class_names[i];
        let mark = if c < 30 { " ← 少ない!" } else { "" };
        println!("  {:>2}: {:>4}枚{}", name, c, mark);
    }

    Ok((pixels_all, labels_all))
}

// ── ミニバッチ作成（シングルスレッド・順序保証）──────────────────────────

fn make_batch<Bk: Backend>(
    pixels:  &[Vec<f32>],
    labels:  &[usize],
    indices: &[usize],
    device:  &Bk::Device,
) -> (Tensor<Bk, 4>, Tensor<Bk, 1, burn::tensor::Int>) {
    let bs = indices.len();
    let mut flat_px  = Vec::with_capacity(bs * 3 * 32 * 32);
    let mut flat_lbl = Vec::with_capacity(bs);
    for &i in indices {
        flat_px.extend_from_slice(&pixels[i]);
        flat_lbl.push(labels[i] as i64);
    }
    let x = Tensor::<Bk, 1>::from_floats(flat_px.as_slice(), device)
        .reshape([bs, 3, 32, 32]);
    let y = Tensor::<Bk, 1, burn::tensor::Int>::from_ints(
        flat_lbl.as_slice(), device,
    );
    (x, y)
}

// ── 学習ループ ─────────────────────────────────────────────────────────────

fn train_model(
    dataset_dir:     &str,
    model_save_path: &str,
    epochs:          usize,
    batch_size:      usize,
    lr:              f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let device: <B as Backend>::Device = Default::default();

    println!("データセット読み込み中: {}", dataset_dir);
    let (pixels, labels) = load_dataset(dataset_dir)?;
    let n = pixels.len();
    println!("合計サンプル数: {}", n);

    let mut model: HexCnn<B> = HexCnn::new(&device);
    let mut optim = AdamConfig::new().init();
    let mut indices: Vec<usize> = (0..n).collect();
    let mut rng = thread_rng();

    for epoch in 0..epochs {
        // ③ 学習率スケジュール: 前半30epochは lr、後半は lr/10
        let current_lr = if epoch < 30 { lr } else { lr * 0.1 };

        indices.shuffle(&mut rng);
        let mut total_loss = 0f32;
        let mut n_batches  = 0usize;

        for chunk in indices.chunks(batch_size) {
            let (x, y) = make_batch::<B>(&pixels, &labels, chunk, &device);
            let logits  = model.forward_train(x);
            let loss    = CrossEntropyLoss::new(None, &logits.device())
                            .forward(logits, y);
            let loss_val = loss.clone().into_scalar();
            let grads    = loss.backward();
            let grads    = GradientsParams::from_grads(grads, &model);
            model = optim.step(current_lr, model, grads);
            total_loss += loss_val;
            n_batches  += 1;
        }

        println!(
            "Epoch {:>3}/{}: avg_loss = {:.4}  lr={:.2e}",
            epoch + 1, epochs,
            total_loss / n_batches as f32,
            current_lr,
        );
    }

    model.save_file(model_save_path, &CompactRecorder::new())
        .map_err(|e| format!("モデル保存失敗: {:?}", e))?;
    println!("モデル保存完了: {}", model_save_path);
    Ok(())
}

// ── エントリポイント ───────────────────────────────────────────────────────

fn main() {
    let dataset_dir     = "dataset-manual-3ch";
    let model_save_path = "hex_cnn_model";
    let epochs          = 50;  // ② 30→50に増加
    let batch_size      = 32;
    let lr              = 1e-3;

    match train_model(dataset_dir, model_save_path, epochs, batch_size, lr) {
        Ok(())  => println!("学習完了"),
        Err(e)  => eprintln!("エラー: {}", e),
    }
}
