# 桌面宠物跨平台架构方案

## 1. 技术栈定案

推荐方案：

- `Tauri 2`
- `Rust`
- `Svelte + TypeScript`

目标：

- Mac / Windows 共用一套产品逻辑
- 保持轻量级，不走 Electron
- 宠物行为逻辑独立于 UI
- 未来可扩展到 Linux

## 2. 为什么这样拆

桌面宠物不是普通网页应用，它有几个很明确的系统能力诉求：

- 透明窗口
- 始终置顶
- 可拖拽悬浮
- 菜单栏 / 托盘入口
- 本地提醒通知
- 开机启动

这些能力如果直接和页面逻辑绑在一起，后面做 Windows 版本会很痛。

所以建议把项目拆成三层：

### 核心层

负责所有和平台无关的逻辑。

- 宠物状态机
- 拖拽物理
- 回弹与重力
- 番茄钟计时
- 喝水提醒
- 站立提醒
- 配置读写

### 表现层

负责页面和动画展示。

- 宠物渲染
- 面板 UI
- 文案展示
- 动画触发

### 平台层

负责系统窗口和设备能力。

- 透明无边框窗口
- 置顶
- 托盘
- 通知
- 开机启动
- 屏幕信息

## 3. 推荐目录结构

```text
pet/
  apps/
    desktop/
      src/
        main/                 # Tauri / Rust 主进程入口
        ui/                   # Svelte 前端
        assets/               # 宠物图、动画帧、音效
        components/
        stores/
        styles/
      src-tauri/
        src/
          main.rs
          app.rs
          physics/
          pet/
          timer/
          reminders/
          config/
          platform/
        Cargo.toml
  crates/
    core/                     # 可选：纯 Rust 核心逻辑库
    protocol/                 # 前后端消息协议
```

如果你想先做得更简单，也可以只保留一个桌面应用目录：

```text
pet-desktop/
  src/
  src-tauri/
```

但从长期维护角度看，建议至少把 `core` 和 `desktop` 分开。

## 4. 关键模块设计

### 4.1 宠物状态机

宠物状态不要靠 UI 的零散布尔值控制，应该统一成状态机。

建议状态：

- `idle`
- `hover`
- `dragging`
- `bouncing`
- `falling`
- `landing`
- `focused`
- `hydrated`
- `standing_reminder`

状态机负责的事情：

- 决定当前宠物动作
- 决定播放哪段动画
- 决定提示文案
- 决定面板里的状态展示

### 4.2 物理模块

这个模块负责：

- 鼠标拖拽跟随
- 横向弹性
- 纵向重力
- 松手下落
- 落地回弹
- 边缘吸附

建议实现为一个纯函数或小型状态更新器，输入：

- 当前时间
- 鼠标位置
- 鼠标速度
- 屏幕边界

输出：

- 宠物位置
- 宠物速度
- 缩放比例
- 旋转角度
- 当前状态

### 4.3 番茄钟模块

职责：

- 开始 / 暂停 / 恢复 / 结束
- 计算剩余时间
- 触发阶段切换
- 发出“专注完成”事件

建议事件：

- `focus_started`
- `focus_tick`
- `focus_warning`
- `focus_completed`

### 4.4 提醒模块

喝水和站立提醒都可以统一成“提醒调度器”。

输入：

- 当前时间
- 用户配置
- 上次确认时间

输出：

- 是否需要提醒
- 提醒类型
- 提醒文案
- 宠物表现状态

### 4.5 配置模块

建议支持：

- 番茄钟默认时长
- 喝水间隔
- 站立间隔
- 宠物位置
- 是否开机启动
- 是否总在最前
- 当前宠物皮肤 / IP 配置

配置落地方式：

- 早期用本地 JSON
- 后期如果数据复杂再切 SQLite

## 5. 前后端消息协议

Tauri 的前后端通信最好不要散乱地随手写事件名。

建议定义统一协议，比如：

```ts
type AppEvent =
  | { type: "pet:drag_start"; payload: { x: number; y: number } }
  | { type: "pet:drag_move"; payload: { x: number; y: number; dx: number; dy: number } }
  | { type: "pet:drag_end"; payload: { x: number; y: number; vx: number; vy: number } }
  | { type: "timer:start"; payload: { focusMinutes: number; breakMinutes: number } }
  | { type: "timer:pause" }
  | { type: "reminder:water_due" }
  | { type: "reminder:stand_due" };
```

这样做的好处：

- 前端不会随便依赖 Rust 内部结构
- 后面换 UI 框架成本低
- 日后加 Windows / Linux 不需要重写业务协议

## 6. 轻量级策略

你明确说希望轻量级，所以要避免几个常见坑。

### 不建议

- 不要用 Electron
- 不要在前端堆一整套重型全局状态管理
- 不要把所有动画都做成复杂视频资源
- 不要把每个宠物状态都拆成大量组件树

### 建议

- 宠物画面尽量用轻量 PNG 序列帧或简单 SVG / Canvas
- 动画优先用 CSS transform
- 逻辑优先放 Rust
- 配置和消息体保持小而清晰

## 7. Mac / Windows 差异处理

你要跨平台优先，但又要轻量，所以平台差异应该只存在于最外层。

### 共用

- 宠物行为
- 物理运动
- 计时器逻辑
- 提醒逻辑
- 配置结构

### Mac 特有

- 菜单栏体验
- 原生通知样式
- 窗口透明和穿透优化

### Windows 特有

- 托盘交互
- 开机启动注册表 / 启动项
- 通知权限细节

原则：

- 共用逻辑不分叉
- 平台差异封装在 `platform/` 下
- UI 上尽量保持一套设计

## 8. 宠物动画实现建议

对于你这个 IP 宠物，我建议首版动画不要太复杂。

### 资源形式

- 2 到 4 个方向状态
- 每个状态 6 到 12 帧
- 先做待机、拖动、落地、庆祝四类

### 动画优先级

1. 拖动和弹跳反馈
2. 松手下落和落地
3. 番茄钟专注态
4. 喝水 / 站立提醒态

### 为什么这样排

桌宠的“活感”主要来自动作反馈，不是高帧率大动画。
先把动作打通，比堆细节更重要。

## 9. DIY 宠物入口预留

你后面想开放用户自定义宠物，这个架构天然适合。

可以把 DIY 拆成几个可配置项：

- 形状
- 颜色
- 主元素
- 眼睛
- 嘴型
- 装饰
- 主题粒子
- 触发音效

以后用户的 DIY 内容，本质上就是一份宠物配置 JSON，再由 `imagegen` 或本地渲染管线生成资产。

## 10. 首批里程碑

### Milestone 1

- Tauri 桌面窗口跑起来
- 宠物悬浮显示
- 拖拽移动
- 松手下落
- 轻微弹性

### Milestone 2

- 番茄钟
- 喝水提醒
- 站立提醒
- 宠物状态切换

### Milestone 3

- 面板 UI 完整化
- 用户设置
- 宠物皮肤 / IP 接入

### Milestone 4

- DIY 宠物入口
- imagegen 生成流程
- 多平台补齐

## 11. 当前建议的落地顺序

如果现在就开干，建议按这个顺序：

1. 先搭 `Tauri + Svelte + Rust` 最小壳子
2. 先把宠物拖拽和物理做出来
3. 再接面板 UI
4. 再接番茄钟和提醒
5. 最后再做 IP 美术和 imagegen 流程

