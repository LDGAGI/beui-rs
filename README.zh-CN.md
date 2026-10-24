# beui-rs

[English](README.md) · **简体中文**

[beUI](https://github.com/starc007/ui-components) 的 [Dioxus](https://dioxuslabs.com) 0.7 移植版——同样的外观，同样的动效，用 Rust 实现。

beUI 是 Saurabh Chauhan 的 React/Next.js 动效组件库（MIT）。本移植把它的组件重新实现到 Dioxus，并逐字保留设计 token、Tailwind 类名、缓动曲线和弹簧参数，使两版逐帧一致。

## 安装

三种方式，从最轻的开始。

**先看效果。** gallery 由 CI 构建并部署，不需要本地工具链。
<a href="https://ldgagi.github.io/beui-rs/">ldgagi.github.io/beui-rs</a>

**放进现有 Dioxus 应用。** crate 还没发布到 crates.io，所以直接依赖仓库——`cargo add` 支持这种写法：

```sh
cargo add --git https://github.com/LDGAGI/beui-rs beui
cargo add --git https://github.com/LDGAGI/beui-rs beui-theme -F prebuilt-css
```

它会写入：

```toml
beui = { git = "https://github.com/LDGAGI/beui-rs", package = "beui" }
beui-theme = { git = "https://github.com/LDGAGI/beui-rs", package = "beui-theme", features = ["prebuilt-css"] }
```

`prebuilt-css` 自带编译好的组件样式表，所以不需要 Tailwind、Node 或 Bun。然后：

```rust
use beui::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_effect(beui_theme::install_standalone);
    rsx! { Button { variant: ButtonVariant::Primary, "Save changes" } }
}
```

`install_standalone` 在启动时注入组件工具类和十一套配色。它只覆盖组件库自己的类名，不覆盖你写的页面标记——如果要用自己的 Tailwind 工具类，走下面的 `@source` 方案。

需要可复现构建时，固定一个 revision：

```toml
beui = { git = "https://github.com/LDGAGI/beui-rs", rev = "<sha>", package = "beui" }
```

**从模板开始。**

```sh
cargo install cargo-generate           # 只需一次
cargo generate --git https://github.com/LDGAGI/beui-rs templates/starter
cd beui-app && dx serve --platform web
```

也可以直接手抄 [`templates/starter`](templates/starter)。

### 已经在用 Tailwind

跳过 `prebuilt-css`，让 Tailwind 扫描 crate 源码，用自己的构建。git 依赖位于 `~/.cargo/git/checkouts`：

```sh
find ~/.cargo/git/checkouts -maxdepth 4 -type d -path '*/crates/beui' | head -1
```

```css
@import "tailwindcss";
@import "…/crates/beui-theme/assets/globals.tailwind.css";
@source "…/crates/beui";
@source "./src";
```

这就是 gallery 的做法，也是唯一能让「你自己的工具类 + 组件」一次编译出来的方式。路径里带随机器变化的 checkout 哈希，所以默认方案是预编译样式表。

### 字体

`@font-face` 指向 `/assets/fonts/*.woff2`。把 `beui-theme/assets/fonts/` 复制到应用的 public 目录，或在编译时用 `BEUI_FONT_BASE=https://cdn.example.com/geist` 指向别处。没有这些文件时字体会回退到系统 sans-serif，不会出错。

## 目录结构

| 路径 | 内容 |
|---|---|
| `crates/beui-theme` | `app/globals.css` 原样副本、11 套配色、自托管 Geist / Geist Mono |
| `crates/beui-core` | `cn!`、缓动与弹簧 token、hooks、touch、presence、FLIP、drag、portal，以及 `platform`——两个渲染器唯一分叉的地方 |
| `crates/beui` | `components/motion` 的移植 |
| `crates/beui-agents` | `components/agents` 的移植 |
| `crates/beui-charts` | `components/charts` 的移植 |
| `gallery/` | 组件浏览器：每个组件一页，文案和结构与 beUI 预览一致 |
| `verify/` | Playwright 验证工具：计算样式与动效断言、像素 diff、端到端页面冒烟 |
| `templates/starter` | 可直接复制的应用模板，走预编译样式，不需要 Tailwind 构建 |
| `scripts/` | fixture、registry 和预编译样式表的生成脚本 |

## 环境要求

| 工具 | 版本 |
|---|---|
| Rust | 1.85+（开发时用的是 1.90） |
| Dioxus CLI | 0.7.10 —— `cargo install dioxus-cli --version 0.7.10` |
| Bun | 任意较新版本（验证工具、fixture 生成） |

`dx` 会在首次构建时下载匹配的 Tailwind v4 和 `wasm-bindgen-cli`。

Bun 和 Tailwind 只用于本仓库的 gallery 和验证工具。发布出去的 crate 两者都不需要：`beui-theme` 的 `prebuilt-css` feature 自带编译好的样式表。

## 运行 gallery

```sh
cd gallery
dx serve --platform web        # http://localhost:8080
dx serve --platform desktop    # 原生窗口
```

生产构建：

```sh
dx build --platform web --release
dx build --platform desktop
```

## 检查

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --target wasm32-unknown-unknown -p beui-core -p beui-theme   # 仅 wasm 的代码路径

cd gallery && dx build --platform web && dx build --platform desktop

cd verify && bun install
bun run all     # foundation + hooks + presence + flip + drag + portal + motion
bun run e2e     # 每个 gallery 页面都能启动并绘制（需先构建 web 产物）
```

`beui-theme` 的逐字对比测试会在能找到 beUI 源码时运行。把 `BEUI_SOURCE` 指向它（或把它放在本仓库旁边）即可；否则跳过。

## 如何保持保真

- **样式。** `globals.css` 逐字节复制。组件类名从源码逐字抄录；`cn!` 只做拼接，使用者的 `class` 放在最后，让 CSS 层叠处理冲突。
- **动效。** 弹簧的 `stiffness` / `damping` / `mass` 逐字复制。`visualDuration + bounce` 用 Motion 自己的公式换算，换算后的数值写在调用旁边。
- **布局动画。** `layout` / `layoutId` 一律走 `beui-core::flip`。
- **拖拽。** 指针捕获和速度采样在 `beui-core::drag` 里。
- **出场。** `AnimatePresence` 对应 `beui-core::presence`；动画结束才卸载组件。
- **减少动效。** 双轨：全局 CSS 规则 + 每个组件里的 `use_reduced_motion()`。
- **图标。** Lucide 路径数据，生成到 `crates/beui/src/icons.rs`。

有三处不是 1:1 翻译，已在 crate 文档里说明：DOM 访问（`beui-core::platform`）、逐帧动画写入（走响应式 `style` 属性）、`cn()`（有意不做 `tailwind-merge`）。

## 状态

motion、agents、charts 三套组件都已完成移植。gallery 已覆盖每个 motion/blocks 组件；charts 和 agents 的专属页面还在补，但两个 crate 本身是完整的。`shader-background` 是唯一的 `n/a`：源码是 WebGL shader，本移植保留 DOM 契约并用 CSS 占位。

## 许可证

MIT，见 [LICENSE](LICENSE)。移植包含来自 beUI、Geist 和 Lucide 的内容，见 [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)。
