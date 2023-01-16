# create-x

## Install via Cargo

```shell
cargo install --git https://github.com/nomyfan/create-x
```

## Initialize a project from a template

Copy the url in browser address bar targeting the template folder and provide the project name.

### GitHub

```shell
create-x \
--url https://github.com/nomyfan/templates/tree/main/vite-react-ts \
--name vite-react-ts
```

```shell
create-x \
--url https://your-github-domain.com/nomyfan/templates/tree/main/vite-react-ts \
--name vite-react-ts \
--type github
```

### GitLab

```shell
create-x \
--url https://gitlab.com/nomyfan/templates/-/tree/main/vite-react-ts \
--name vite-react-ts
```

```shell
create-x \
--url https://your-gitlab-domain.com/nomyfan/templates/-/tree/main/vite-react-ts \
--name vite-react-ts \
--type gitlab
```
