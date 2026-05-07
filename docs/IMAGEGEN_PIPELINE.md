# Imagegen 资产生成流程

## 目标

为桌面宠物建立一套可复用的图像生成流程。

要求：

- 固定 IP 版先产出主形象
- 后续可以扩展为用户 DIY 生成
- 输出适合桌宠使用的透明背景资产

## 资产类型

### 1. 主形象

用于桌面常驻显示。

需要：

- 正面主视图
- 待机状态
- 开心状态
- 疲惫状态
- 提醒状态

### 2. 动作帧

用于简单动画。

需要：

- 拖动起身
- 左右弹跳
- 落地回弹
- 庆祝
- 呼吸

### 3. 面板插图

用于点击后的小面板。

需要：

- 头像
- 状态图标
- 提醒图标
- 小装饰元素

## 生成原则

- 统一比例
- 统一光照
- 统一材质
- 统一边缘处理
- 尽量用透明背景

## 推荐风格约束

如果你的 IP 允许，建议把宠物资产统一到以下方向：

- 可爱
- 圆润
- 柔软
- 轻微毛绒感或果冻感
- 元素气息明确但不过度复杂

## Prompt 结构

建议把提示词拆成五段：

1. 角色来源
2. 形态特征
3. 材质与色彩
4. 动作状态
5. 输出要求

### 示例模板

```text
[角色来源/参考IP],
桌面宠物化设计,
圆润、可爱、轻量化,
透明或浅色背景,
柔和发光,
元素风格,
高识别度的眼睛和表情,
适合桌面常驻显示,
单体居中,
无文字,
无多余背景物,
PNG透明背景,
角色正面视图
```

## 动作帧模板

### 拖动状态

```text
same character,
dragging motion,
slight squash and stretch,
soft bounce,
dynamic tilt,
transparent background,
single character centered
```

### 落地状态

```text
same character,
landing bounce,
bottom squash,
soft impact,
transparent background,
single character centered
```

### 站立提醒状态

```text
same character,
stretching pose,
slightly raised body,
alert but cute expression,
transparent background,
single character centered
```

## DIY 入口对应的参数

后续用户在 UI 里可以暴露这些参数：

- 主色
- 次色
- 眼睛样式
- 嘴型
- 头顶装饰
- 元素类型
- 材质风格
- 情绪风格

这些参数最后都可以拼成 prompt，交给 `imagegen` 生成。

## 输出规范

建议统一输出：

- PNG
- 透明背景
- 1024x1024 或更高
- 单角色居中
- 边缘干净

## 版本建议

### v1

- 先出固定 IP 的 4 个基础状态

### v2

- 加 6 到 8 个动作帧

### v3

- 接入 DIY 参数化生成

