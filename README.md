# SensitiveInfoExtractor

<div align="center">

**Rust 敏感信息提取工具**

![SensitiveInfoExtractor](https://socialify.git.ci/xihan123/sensitive-info-extractor/image?description=1&forks=1&issues=1&language=1&name=1&owner=1&pulls=1&stargazers=1&theme=Auto)
[![CI/CD](https://github.com/xihan123/sensitive-info-extractor/actions/workflows/build-release.yml/badge.svg)](https://github.com/xihan123/sensitive-info-extractor/actions/workflows/build-release.yml)
[![Release](https://img.shields.io/github/v/release/xihan123/sensitive-info-extractor)](https://github.com/xihan123/sensitive-info-extractor/releases)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

[下载](#下载) • [快速开始](#快速开始) • [编译](#从源码编译)

</div>

---

## 简介

从 Excel 表格里提取手机号、身份证号、银行卡号。支持 xlsx/xls，多文件并行处理，拖拽操作。身份证用校验码验证，银行卡用 Luhn
算法，手机号验证号段。导出的 Excel 报告里有效信息标绿、无效标红。

---

## 下载

从 [GitHub Releases](https://github.com/xihan123/sensitive-info-extractor/releases/latest) 下载。

| 平台                  | 文件名                                                   |
|---------------------|-------------------------------------------------------|
| Windows x64         | `sensitive-info-extractor-windows-x64.exe`            |
| Windows x64 (压缩版)   | `sensitive-info-extractor-windows-x64-compressed.exe` |
| macOS Intel         | `sensitive-info-extractor-macos-x64`                  |
| macOS Apple Silicon | `sensitive-info-extractor-macos-arm64`                |
| Linux x64           | `sensitive-info-extractor-linux-x64`                  |

标准版约 10MB，压缩版约 5MB，无需安装依赖。

---

## 快速开始

### Windows

```powershell
sensitive-info-extractor-windows-x64.exe
# 或直接拖文件到 exe 上
```

### macOS / Linux

```bash
chmod +x sensitive-info-extractor-*
./sensitive-info-extractor-*
```

### 操作步骤

拖文件进去 → 选列 → 点开始 → 导出

程序会自动识别"消息内容"、"内容"、"短信"这类列名。

---

## 从源码编译

需要 Rust 1.75+ 和 Git。

```bash
git clone https://github.com/xihan123/sensitive-info-extractor.git
cd sensitive-info-extractor
cargo build --release
```

跨平台编译加上 `--target` 参数即可。

---

## 配置

- **目标列**：要提取的列名，默认"消息内容"
- **上下文行数**：提取时带上前后几行，默认 2 行
- **提取类型**：手机号/身份证/银行卡，可以单独开关

输出 Excel 特性：表头蓝底白字、首行冻结、自动筛选、列宽自适应。

---

## 开发

```bash
cargo test                              # 跑测试
cargo test test_validate_id_card        # 身份证校验测试
cargo test test_validate_bank_card      # 银行卡测试
```

项目结构：

```
src/
├── main.rs           # 入口
├── core/             # 读取、提取、校验、导出
├── gui/              # 界面组件
├── models/           # 数据结构
└── utils/            # 正则、工具函数
```

---

## 校验算法

**身份证**：18位校验码 + 出生日期验证  
**银行卡**：Luhn 算法（偶数位乘2，大于9减9，求和模10）  
**手机号**：1开头，第二位3-9，支持带分隔符和+86

---

## 致谢

- [eframe/egui](https://github.com/emilk/egui) - GUI
- [calamine](https://github.com/tafia/calamine) - Excel 读取
- [rust_xlsxwriter](https://github.com/jmcnamara/rust_xlsxwriter) - Excel 写入
- [rayon](https://github.com/rayon-rs/rayon) - 并行处理

---

## 免责声明

仅用于合法用途，使用者自行承担责任。

---

<div align="center">

觉得有用就点个 Star 吧

[⬆ 返回顶部](#sensitiveinfoextractor)

</div>
