# 1. イメージのビルド
docker build -t ksnctf-esp-local .

# 2. コンテナの起動（10777ポートをバインド タイムゾーンをニューヨークに！）
docker run -d -p 10777:10777 -e TZ=America/New_York --name esp-challenge ksnctf-esp-local