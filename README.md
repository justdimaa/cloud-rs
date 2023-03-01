# cloud-rs
This project is currently under development and not yet ready for use in any environments. 

## What works so far:
- Linux support (Windows and macOS untested)
- User authentication
- Downloading, uploading, and replacing files

## Setup
1. Create a `.env` file in the workspace directory, with the following variables:
```
# client
DATABASE_URL=sqlite:///path/to/local/.sync.db # only required for client builds

# api
API_DATABASE_URL=mongodb://root:yourmongopassword@localhost:27017
API_URL=[::1]:50051

# docker
DOCKER_MONGO_USER=root
DOCKER_MONGO_PWD=yourmongopassword
```

2. If you don't have a mongodb server running, use this command:
```
# docker and docker-compose must be installed on your system
# a new mongodb server will be set up with the .env credentials
docker-compose up -d
```

3. Start the api server:
```
cargo run --bin cloud-api
```

4. Open the client:
```
cargo run --bin cloud-desktop
```
