**note to self**


1. Execute Dockerfile for build with any image name to build image
```
docker image build -t [BUILD_IMAGE_TAG] -f Dockerfile.build . 
```

2. Run the image and create a binary file in `/target/release/`.
```
docker container run --rm \                                                                               
    -v $PWD:/code \
    -v $HOME/.cargo/registry:/root/.cargo/registry \
    -v $HOME/.cargo/git:/root/.cargo/git \
    [BUILD_IMAGE_TAG]
```

3. Build the image for runtime
```
docker build -t [RUNTIME_IMAGE_TAG] .
```

4. Obtain authentication token and authenticate Docker client against registry
https://docs.aws.amazon.com/AmazonECR/latest/userguide/Registries.html#registry_auth

5. Push Docker image to ECR
https://docs.aws.amazon.com/AmazonECR/latest/userguide/docker-push-ecr-image.html

6. Deploy ECR image with Labmda

7. Run ECR image with Lambda
```
aws lambda invoke --invocation-type Event --function-name [FUNCTION_NAME] --region [REGION_NAME] --payload '{"bucket_name":"xxx", "prefix":"xxx", "tgt_size":128, "tgt_ext":"png"}' --cli-binary-format raw-in-base64-out outputfile.txt
```
