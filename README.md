# eegy (WIP) 
An easy EEG signal processing library in Rust.

## 概要
1. ノードエディタで脳波解析フローを作成する
2. 作成したフローをJson等にシリアライズする 
3. Unity等の実環境から作成したフローを動作させる

ことで、リアルタイム脳波解析の試行錯誤を減らすことを目的にしています。

## クレート構成
- `crates/eegy-core`(WIP)
  - 脳波処理コア機能 
  - グラフエンジン 
  - シリアライズ/デシリアライズ機能
- `crates/eegy-gui`(WIP)
  - GUI
- `crates/eegy-device`(WIP)
  - 脳波デバイス抽象化レイヤー
- `crates/eegy-simulator`(WIP)
  - テスト・検証用の脳波シミュレーター

## 使用方法
- GUIの起動  
    `cargo run -p eegy-gui`

## 連携周り
- Unity(WIP)
  - UPMによる連帯
  - https://github.com/Shimo888/eegy.git?path=eegy-unity/Assets/Eegy