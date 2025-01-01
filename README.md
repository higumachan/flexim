# Flexim

Fleximは、RustとPythonで構築された高性能なデータ可視化・分析プラットフォームです。
大規模なデータセットの効率的な可視化、リアルタイムデータの表示、インタラクティブな分析機能を提供します。

## 主な特徴

- **マルチ言語サポート**: RustとPythonの両方のインターフェースを提供
- **高性能な可視化**: 大規模データセットの効率的な可視化
- **リアルタイムデータ処理**: gRPCベースのクライアント-サーバーアーキテクチャ
- **柔軟なレイアウト**: カスタマイズ可能なタイル型UIシステム
- **多様なデータタイプ**: 画像、テンソル、データフレーム、オブジェクトなどをサポート

## インストール方法

### Pythonパッケージ（クライアント）
```bash
pip install flexim
```

### Rustクレート
Cargo.tomlに以下を追加：
```toml
[dependencies]
flexim = { git = "https://github.com/higumachan/flexim" }
```

## 使い方

### Pythonでの基本的な使用例
```python
from flexim_sdk.client import Client
from flexim_sdk.bag import Bag

# クライアントの初期化
client = Client()

# データバッグの作成
bag = client.create_bag("my_data")

# データの追加
bag.append_dataframe(df)  # Pandasデータフレーム
bag.append_image(img)     # 画像データ
bag.append_tensor(tensor) # 2次元テンソル
```

### Rustでの基本的な使用例
```rust
use flexim::client::Client;
use flexim::bag::Bag;

// クライアントの初期化
let client = Client::new();

// データバッグの作成
let bag = client.create_bag("my_data")?;

// データの追加
bag.append_dataframe(&df)?;  // DataFrameの追加
bag.append_image(&img)?;     // 画像の追加
bag.append_tensor(&tensor)?; // テンソルの追加
```

## クイックスタート

1. インストール
```bash
pip install flexim
```

2. データの可視化
```python
from flexim_sdk.client import Client

# クライアントの初期化と可視化の開始
client = Client()
bag = client.create_bag("my_visualization")

# データの追加（例：Pandasデータフレーム）
bag.append_dataframe(df)
```

## サポートされているデータタイプ

Fleximは以下のデータタイプの可視化をサポートしています：

- **データフレーム**: 表形式データの表示と分析
- **画像**: 単一画像やバッチ画像の表示
- **2次元テンソル**: ヒートマップなどの可視化
- **オブジェクト**: カスタムデータ型の可視化

## システム概要

Fleximは、高性能なRustエンジンと使いやすいPython APIを組み合わせることで、
大規模データの効率的な可視化と分析を実現します。カスタマイズ可能なタイル型UIにより、
複数のデータビューを柔軟にレイアウトできます。

## ライセンス


このプロジェクトのライセンスについては、[LICENSE](LICENSE)ファイルをご確認ください。
