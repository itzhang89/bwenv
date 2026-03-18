# bwenv - Bitwarden to Environment Variables Tool

从 Bitwarden vault 读取凭据并转换为环境变量的 CLI 工具。

## 安装

```bash
# 克隆项目
git clone <repo-url>
cd bwenv

# 构建
cargo build --release

# 安装到 PATH
cp target/release/bwenv /usr/local/bin/
```

## 前置要求

- 系统已安装 [Bitwarden CLI](https://github.com/bitwarden/clients/releases) (`bw`)
- 首次使用需先登录: `bw login`

## 快速开始

```bash
# 1. 创建项目配置文件 ~/.bwenv
# 2. 加载项目
bwenv project load ~/.bwenv

# 3. 直接使用项目（自动切换并导出环境变量）
bwenv use myproject

# 4. 或者直接运行（使用当前项目）
bwenv
```

## 项目配置文件

创建 `~/.bwenv` 文件：

```yaml
# ~/.bwenv
- name: "dev"
  prefix: "dev"
  services:
    - mysql
    - redis
    - github

- name: "prod"
  prefix: "prod"
  services:
    - mysql
    - postgres
```

加载项目：

```bash
bwenv project load ~/.bwenv/projects
```

## 使用方式

### 方式1：直接运行（使用当前项目）

```bash
# 默认运行 generate 命令
bwenv

# 指定输出文件
bwenv -o .env
```

### 方式2：使用项目

```bash
# 切换到项目并导出环境变量
bwenv use dev

# 指定输出格式
bwenv use dev -f json -o secrets.json

# 交互式选择服务
bwenv use dev --select
```

### 方式3：命令行参数

```bash
# 指定项目
bwenv --project prod

# 指定服务
bwenv -s github -s aliyun

# 指定前缀
bwenv -p thoughtworks
```

## 命令

### bwenv（默认）

直接运行 generate 命令：

```bash
bwenv                          # 使用当前项目
bwenv -o .env                  # 导出到文件
bwenv -s github                # 指定服务
bwenv -p dev                   # 指定前缀
bwenv -f json                  # JSON 格式
```

### use

切换到指定项目并导出环境变量：

```bash
bwenv use dev                  # 切换并导出
bwenv use prod -o .env        # 切换并导出到文件
```

### project

项目管理：

```bash
bwenv project                  # 列出项目
bwenv project list             # 列出项目
bwenv project add dev "dev" "mysql,redis"    # 添加项目
bwenv project load ~/.bwenv           # 从文件加载项目
bwenv project remove dev      # 删除项目
bwenv project use dev         # 设置当前项目
```

### 其他命令

```bash
bwenv list                    # 列出 items
bwenv current                 # 查看当前项目
bwenv config show             # 显示配置
bwenv config init             # 初始化配置
```

## 配置

### 主配置文件

`~/.bwenv/config.yaml`:

```yaml
bitwarden:
  master_password: "your-master-password"

default_format: "shell"

# 当前选中的项目
current_project: "dev"
```

### 项目配置文件

`~/.bwenv`:

```yaml
- name: "dev"
  prefix: "dev"
  services:
    - mysql
    - redis

- name: "prod"
  prefix: "prod"
  services:
    - mysql
```

### Master Password 优先级

1. 环境变量 `BW_MASTER_PASSWORD`
2. 配置文件
3. 运行时输入

### 项目目录 .bwenv 文件

在项目目录中创建 `.bwenv` 文件，可以自动检测并切换到该项目：

```yaml
# 项目目录下的 .bwenv
name: "myproject"
prefix: "dev"
services:
  - mysql
  - redis
```

运行 `bwenv` 时，如果当前目录或父目录存在 `.bwenv` 文件，会自动选择该项目。

## 使用示例

### 开发环境

```bash
# 方式1：直接导出
bwenv use dev -o .env
source .env

# 方式2：直接 eval
eval $(bwenv use dev)
```

### Docker Compose

```bash
bwenv use prod -o .env
```

### CI/CD

```bash
export BW_MASTER_PASSWORD="$BW_MASTER_PASSWORD"
bwenv use prod -f json > secrets.json
```

## 环境变量

| 变量 | 说明 |
|------|------|
| `BW_MASTER_PASSWORD` | Bitwarden 主密码 |
