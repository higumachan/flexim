# Flexim

Fleximは、データの可視化と操作のためのRustで書かれたツールキットです。

## 概要

Fleximは以下の機能を提供する複数のクレートで構成されています：

- **flexim-data-type**: データ型の定義と管理
- **flexim-data-view**: データの表示と可視化コンポーネント
- **flexim-data-visualize**: データの可視化機能
- **flexim-polars**: [Polars](https://www.pola.rs/)との統合機能
- **flexim-table-widget**: テーブル表示用ウィジェット
- **flexim-layout**: レイアウト管理システム
- **flexim-connect**: クライアント-サーバー間通信
- **flexim-utility**: 共通ユーティリティ関数
- **flexim-font**: フォント管理システム
- **flexim-storage**: データストレージ機能
- **flexim-config**: 設定管理システム
- **flexim-storybook**: コンポーネントのビジュアルテスト用ストーリーブック

## 機能

- データの可視化と分析
- テーブル形式でのデータ表示
- クライアント-サーバーアーキテクチャのサポート
- カスタマイズ可能なレイアウトシステム
- 日本語フォントのサポート（NotoSansJP）

## 依存関係

- Rust
- Cargo（Rustのパッケージマネージャー）
- その他の依存関係は各クレートの`Cargo.toml`ファイルに記載されています

## ライセンス

このプロジェクトのライセンスについては、[LICENSE](LICENSE)ファイルをご確認ください。
