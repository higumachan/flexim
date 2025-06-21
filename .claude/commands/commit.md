# Claude Codeへのgitmojiコミット指示書

## 指示

以下のルールに従って、すべてのgitコミットメッセージを作成してください：

1. **コミットメッセージの形式**

   ```
   <emoji> <subject>

   <body> (オプション)

   <footer> (オプション)
   ```

2. **基本ルール**

   - コミットメッセージの先頭には必ず適切なgitmojiを配置する
   - subjectは50文字以内で簡潔に記述
   - 変更内容に最も適した絵文字を選択する
   - 複数の変更がある場合は、最も重要な変更を表す絵文字を使用

3. **コミットの粒度**
   - 1つのコミットには1つの論理的な変更のみを含める
   - 関連性のない変更は別々のコミットに分ける

## moji(絵文字) と semanticsの一覧

### 機能追加・改善

- ✨ `:sparkles:` - 新機能の追加
- 🚀 `:rocket:` - パフォーマンス改善
- 💄 `:lipstick:` - UI/スタイルファイルの追加・更新
- ♿️ `:wheelchair:` - アクセシビリティの改善
- 🌐 `:globe_with_meridians:` - 国際化・ローカライゼーション
- 💬 `:speech_balloon:` - テキストやリテラルの追加・更新

### バグ修正

- 🐛 `:bug:` - バグ修正
- 🚑️ `:ambulance:` - 緊急のバグ修正・ホットフィックス
- 🩹 `:adhesive_bandage:` - 簡単な修正（non-critical issues）

### コード品質

- ♻️ `:recycle:` - リファクタリング
- 🎨 `:art:` - コードの構造・フォーマットの改善
- 🔥 `:fire:` - コードやファイルの削除
- 💡 `:bulb:` - ソースコード内のコメント追加・更新
- 🏗️ `:building_construction:` - アーキテクチャの変更
- 🧑‍💻 `:technologist:` - 開発者体験の改善

### ドキュメント

- 📝 `:memo:` - ドキュメントの追加・更新
- 📄 `:page_facing_up:` - ライセンスの追加・更新

### 依存関係・設定

- ➕ `:heavy_plus_sign:` - 依存関係の追加
- ➖ `:heavy_minus_sign:` - 依存関係の削除
- ⬆️ `:arrow_up:` - 依存関係のアップグレード
- ⬇️ `:arrow_down:` - 依存関係のダウングレード
- 📌 `:pushpin:` - 特定バージョンへの依存関係の固定
- 🔧 `:wrench:` - 設定ファイルの追加・更新

### テスト

- ✅ `:white_check_mark:` - テストの追加・更新・合格
- 🧪 `:test_tube:` - 失敗するテストの追加

### CI/CD・インフラ

- 💚 `:green_heart:` - CI ビルドの修正
- 👷 `:construction_worker:` - CIビルドシステムの追加・更新
- 🐳 `:whale:` - Docker関連の変更

### バージョン管理

- 🔖 `:bookmark:` - リリース/バージョンタグ
- 🚧 `:construction:` - 作業中（WIP）
- ⏪️ `:rewind:` - 変更のrevert

### その他

- 🎉 `:tada:` - プロジェクトの開始
- 🙈 `:see_no_evil:` - .gitignoreの追加・更新
- 📦️ `:package:` - コンパイル済みファイルやパッケージの更新
- 🔀 `:twisted_rightwards_arrows:` - ブランチのマージ
- 🔒️ `:lock:` - セキュリティの修正
- 🚨 `:rotating_light:` - コンパイラ/リンターの警告の修正

### 使用例

```bash
# 新機能追加
✨ ユーザー認証機能を実装

# バグ修正
🐛 ログイン時のセッション管理エラーを修正

# ドキュメント更新
📝 READMEにインストール手順を追加

# リファクタリング
♻️ UserServiceクラスのメソッドを整理

# 依存関係の更新
⬆️ React 18.2.0にアップグレード

# 開発者体験の改善
🧑‍💻 VSCode用のデバッグ設定を追加
```
