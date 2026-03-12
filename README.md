# RusticOS
1510tyが気分で始めた、GUIOSです

なんかたまたまいろいろうまくいっていい感じになりました(語彙力損失)

## ビルド&実行するには?
※適当に書いただけです これだけじゃたぶん動かん あとSecureBootはビルドしたなら切ってね

・QEMUを用意しといてください あとOVMFも

・rustupを行っておいて、nightyにしといてください

・事前にLimineを用意して配置しといてください

1.リポジトリをクローンする

2.cargo build -p kernel --target x86_64-unknown-noneをプロジェクトルートで実行

3.target/x86_64-unknown-none/debug/kernelをコピー

4.Limineを起動するドライブのルートに配置

5.limine.conf(<後日用意>)を配置する

6.起動!
