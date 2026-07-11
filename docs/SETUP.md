# 分平台安装指南

## Windows

### 1. 安装依赖

#### Node.js ≥ 18
```
winget install OpenJS.NodeJS.LTS
```
或从 [nodejs.org](https://nodejs.org) 下载。

#### Rust + MSVC 工具链
```powershell
# 安装 Rust
winget install Rustlang.Rustup
rustup default stable-msvc

# 安装 Visual Studio Build Tools
# 从 https://visualstudio.microsoft.com/visual-cpp-build-tools/ 下载
# 安装时选择 "Desktop development with C++" 工作负载
```

> **注意**：Rust MSVC 编译需要 `link.exe`（MSVC 链接器）。如果 Git Bash 的 `/usr/bin/link` 遮蔽了 MSVC 的 `link.exe`，请在 **Developer Command Prompt for VS 2022** 中运行 `cargo check`。

#### Python ≥ 3.10
```
winget install Python.Python.3.11
```

#### Ollama
```
winget install Ollama.Ollama
# 或从 https://ollama.com 下载安装包

# 拉取模型
ollama pull gemma4:12b
```

### 2. 配置项目

```powershell
# 克隆项目
cd ai-companion
npm install

# Python 依赖
pip install -r workers/requirements.txt
```

### 3. 下载模型（可选）

```powershell
# 查看可用模型
python workers/download_models.py --list

# 下载 Embedding 模型（RAG 需要）
python workers/download_models.py bge-m3

# 下载 ASR 模型（语音需要）
python workers/download_models.py whisper

# 下载 TTS 模型
python workers/download_models.py cosyvoice
```

### 4. 运行

```powershell
npm run tauri dev
```

---

## macOS

### 1. 安装依赖

```bash
# Homebrew
brew install node rust python@3.11 ollama

# 拉取模型
ollama pull gemma4:12b
```

### 2. 配置项目

```bash
cd ai-companion
npm install
pip3 install -r workers/requirements.txt
```

### 3. 运行

```bash
npm run tauri dev
```

---

## Linux (Ubuntu 22.04+)

### 1. 安装依赖

```bash
# Node.js
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs

# 系统依赖（Tauri 需要）
sudo apt install -y libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev \
    librsvg2-dev patchelf libxdo-dev libssl-dev

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Python
sudo apt install -y python3 python3-pip python3-venv

# Ollama
curl -fsSL https://ollama.com/install.sh | sh
ollama pull gemma4:12b
```

### 2. 配置项目

```bash
cd ai-companion
npm install
python3 -m venv .venv
source .venv/bin/activate
pip install -r workers/requirements.txt
```

### 3. 运行

```bash
npm run tauri dev
```

---

## 常见问题

### Rust 编译报错 `link: extra operand`

Git Bash 的 `/usr/bin/link`（GNU coreutils）遮蔽了 MSVC 的 `link.exe`。

**解决方案**：
1. 打开 **Developer Command Prompt for VS 2022**（开始菜单 > Visual Studio 文件夹）
2. 切换到项目目录运行 `cargo check`
3. 或在 PowerShell 中运行（PATH 优先级不同）

### Ollama 连接失败

```
Error: Ollama request failed
```

**检查**：
```bash
ollama list           # 确认 Ollama 在运行
ollama pull gemma4:12b  # 确认模型已下载
curl http://127.0.0.1:11434/api/tags  # 检查 API 可用
```

### Python Worker 启动失败

```bash
# 检查 Python 版本
python --version   # 需 ≥ 3.10

# 检查依赖
pip list | grep msgpack
pip list | grep requests
```

### macOS 权限

首次运行摄像头或麦克风时，macOS 会弹出权限请求。请在 **系统设置 > 隐私与安全性** 中允许应用访问。

### Linux 音频

需要安装 PulseAudio 或 PipeWire：
```bash
sudo apt install -y pulseaudio pulseaudio-utils
```
