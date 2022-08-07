# note to self



1. 任意のイメージ名でビルド用のDockerfileを実行してイメージビルド
```
docker image build -t "resize-container-build" -f Dockerfile.build . 
```

2. イメージを実行して`/target/release/`にバイナリファイルを作成
```
docker container run --rm \                                                                               
    -v $PWD:/code \
    -v $HOME/.cargo/registry:/root/.cargo/registry \
    -v $HOME/.cargo/git:/root/.cargo/git \
    resize-container-build
```

3. ランタイム用のイメージをビルド
```
docker build -t sharedine/app/batch-resize . 
```

4. 認証トークンを取得し、レジストリに対して Docker クライアントを認証

  https://docs.aws.amazon.com/AmazonECR/latest/userguide/Registries.html#registry_auth

5. DockerイメージをECRにプッシュ

  https://docs.aws.amazon.com/AmazonECR/latest/userguide/docker-push-ecr-image.html

6. LabmdaでECRイメージをデプロイ

7. ECRイメージを実行
```
aws lambda invoke --invocation-type Event --function-name [FUNCTION_NAME] --region [REGION_NAME] --payload '{"bucket_name":"xxx", "prefix":"xxx", "target_size":128}' --cli-binary-format raw-in-base64-out outputfile.txt
```

