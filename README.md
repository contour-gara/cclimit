# cclimit

> Claude Code の利用上限の使用率を 1 行で表示する CLI。

```
$ cclimit
5h 11% 2h15m | 7d 38% 3d17h | fable 68% 3d17h
```

Claude Code の利用上限には、5 時間の枠、7 日の枠、モデル別の 7 日の枠があります。cclimit はそれぞれの使用率とリセットまでの残り時間を並べます。シェルから実行するほか、ステータスバーのような定期的に呼び出す仕組みにも組み込めます。

[特徴](#特徴) • [インストール](#インストール) • [使い方](#使い方) • [トラブルシューティング](#トラブルシューティング)

## 特徴

- 設定不要。Claude Code にログイン済みなら、Claude Code が保存した認証情報を読んで動きます。
- 常駐せず、ファイルも書きません。1 回の実行で 1 回の HTTPS リクエストを送って終わります。
- 出力はプレーンテキストの 1 行だけです。色や記号は呼び出し側で付けられます。
- 失敗の仕方が決まっています。一時的な失敗では標準出力が空になるので、呼び出し側は前回の値を使い続けられます。
- 単一バイナリです。macOS では標準の `security` コマンドで Keychain を読みます。

## 動作環境

- macOS。Apple Silicon と Intel 向けのバイナリを配布します。
- Ubuntu などの Linux。x86_64 向けのバイナリを配布します。認証情報は `~/.claude/.credentials.json` から読みます。

どちらの OS でも、claude.ai の Pro または Max サブスクリプションで Claude Code にログインしている必要があります。

## インストール

### mise

```sh
mise use -g github:contour-gara/cclimit
```

GitHub Releases に添付したバイナリを mise が取得します。

### ソースから

Rust のツールチェーンは mise.toml で固定しています。

```sh
git clone https://github.com/contour-gara/cclimit
cd cclimit
mise install
mise exec -- cargo build --release
./target/release/cclimit
```

## 使い方

### 使用率を表示する

引数なしで実行すると、冒頭の例のような 1 行を標準出力に書いて終了します。

| 項目 | 意味 |
|---|---|
| `5h` | 5 時間の枠 |
| `7d` | 7 日の枠。全モデルの合計 |
| `fable` などの小文字のモデル名 | そのモデルだけの 7 日の枠 |

各項目は「ラベル 使用率 リセットまでの残り時間」の順です。残り時間は `3d17h`、`2h15m`、`45m`、`<1m` のように表します。リセット時刻が分からない枠は残り時間を省きます。

### 終了コード

| 結果 | 標準出力 | 終了コード |
|---|---|---|
| 取得できた | 使用率の 1 行 | 0 |
| ログインし直すべき状態 | `cclimit: login required` | 0 |
| 一時的な失敗 | 空 | 1 |

ログインし直すべき状態とは、認証情報が無いか読めない、トークンが期限切れ、API が 401 か 403 を返した場合です。一時的な失敗とは、通信の失敗、5 秒のタイムアウト超過、200、401、403 のいずれでもない HTTP ステータス、読めない応答、枠が 1 つも読み取れない応答です。どちらの場合も理由を標準エラーに 1 行書きます。

### 診断する

`doctor` は、認証情報の読み取り元と期限、API の応答を人が読める形で表示します。トークンの値は表示しません。

```
$ cclimit doctor
credentials: keychain, expires in 2h26m
api: HTTP 200, limits 5h, 7d, fable
```

## 仕組み

認証情報は、macOS では Keychain の `Claude Code-credentials` を `security` コマンドで読みます。読めなければ `~/.claude/.credentials.json` を読みます。Linux では最初から `~/.claude/.credentials.json` を読みます。トークンの再発行はしません。期限が切れていたら `cclimit: login required` を出し、Claude Code 側でのログインに任せます。

使用率は claude.ai の非公開 API `GET https://api.anthropic.com/api/oauth/usage` から取ります。公式に文書化された API ではないので、予告なく変わる可能性があります。応答に知らない項目があっても無視し、既知の枠が 1 つも無いときだけ失敗にします。

## トラブルシューティング

### `cclimit: login required` と表示される

Claude Code で `/login` を実行してログインし直してください。`cclimit doctor` で、認証情報がどこから読まれ、いつ期限が切れるかを確認できます。

### 何も表示されず終了コードが 1 になる

一時的な失敗です。理由は標準エラーに出ています。

```sh
cclimit; echo "exit=$?"
```

`cclimit doctor` の `api:` 行で、API に到達できているかと HTTP ステータスを確認できます。

## 環境変数

通常は設定不要です。テストと調査のためのもので、`--help` には表示されません。

| 環境変数 | 働き |
|---|---|
| `CCLIMIT_API_BASE` | API の接続先。既定は `https://api.anthropic.com` |
| `CCLIMIT_CREDENTIALS_FILE` | 設定すると Keychain を見ずにこのファイルから認証情報を読む |
| `CCLIMIT_NOW` | 現在時刻を `2026-10-13T19:14:59Z` のような RFC 3339 の形式で固定する |

## 開発

```sh
mise run test   # 単体テストと結合テスト
mise run lint   # cargo fmt --check と cargo clippy
mise run live   # 本物の Keychain と API を使うテスト
```

結合テストは実際のバイナリを起動し、HTTP は httpmock、認証情報は一時ファイル、時刻は `CCLIMIT_NOW` で差し替えます。

CI はプルリクエストと main への push で、macOS と Ubuntu の両方で lint と test を実行します。`mise run release` を実行すると版が上がり、タグが push されます。タグを受けて GitHub Actions が macOS と Linux 向けのバイナリをビルドし、Releases に添付します。

保守は [contour-gara](https://github.com/contour-gara) が行っています。不具合や要望は [Issues](https://github.com/contour-gara/cclimit/issues) に書いてください。

## ライセンス

[CC0 1.0 Universal](LICENSE)
